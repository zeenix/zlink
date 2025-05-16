use core::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};
use futures_util::stream::Stream;
use pin_project_lite::pin_project;
use serde::Deserialize;

use crate::{
    connection::{socket::ReadHalf, ReadConnection},
    reply, Result,
};

pin_project! {
    /// A stream of replies.
    ///
    /// This is the type returned by [`Proxy::call_more`].
    ///
    /// This would be useful for external use as well but we keep it internal because
    /// [`ReadConnection::receive_reply`] gives us anonymous futures so we have to keep the future
    /// type generic here. This can be solved with Boxing but we want to avoid all allocations in
    /// the core library.
    #[derive(Debug)]
    pub(super) struct ReplyStream<'c, Read: ReadHalf, F, Fut> {
        #[pin]
        state: ReplyStreamState<Fut>,
        conn: &'c mut ReadConnection<Read>,
        func: F,
    }
}

impl<'c, Read, F, Fut, Params, ReplyError> ReplyStream<'c, Read, F, Fut>
where
    Read: ReadHalf,
    F: FnMut(&'c mut ReadConnection<Read>) -> Fut,
    Fut: Future<Output = Result<reply::Result<Params, ReplyError>>>,
    Params: Deserialize<'c>,
    ReplyError: Deserialize<'c>,
{
    pub(super) fn new(conn: &'c mut ReadConnection<Read>, func: F) -> Self {
        ReplyStream {
            state: ReplyStreamState::Init,
            conn,
            func,
        }
    }
}

impl<'c, Read, F, Fut, Params, ReplyError> Stream for ReplyStream<'c, Read, F, Fut>
where
    Read: ReadHalf,
    F: FnMut(&'c mut ReadConnection<Read>) -> Fut,
    Fut: Future<Output = Result<reply::Result<Params, ReplyError>>>,
    Params: Deserialize<'c>,
    ReplyError: Deserialize<'c>,
{
    type Item = Result<reply::Result<Params, ReplyError>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if this.state.as_mut().check_init() {
            let conn = unsafe { &mut *(*this.conn as *mut _) };
            this.state.set(ReplyStreamState::Future {
                future: (this.func)(conn),
            });
        }

        let item = match this.state.as_mut().project_future() {
            Some(fut) => ready!(fut.poll(cx)),
            None => panic!("Unfold must not be polled after it returned `Poll::Ready(None)`"),
        };

        this.state.set(ReplyStreamState::Init);
        Poll::Ready(Some(item))
    }
}

pin_project! {
    /// State for `ReplyStream`.
    ///
    /// Based on the [`futures::stream::unfold`] implementation.
    #[project = ReplyStreamStateProj]
    #[project_replace = ReplyStreamStateProjReplace]
    #[derive(Debug)]
    enum ReplyStreamState<R> {
        Init,
        Future {
            #[pin]
            future: R,
        },
        Empty,
    }
}

impl<R> ReplyStreamState<R> {
    fn project_future(self: Pin<&mut Self>) -> Option<Pin<&mut R>> {
        match self.project() {
            ReplyStreamStateProj::Future { future } => Some(future),
            _ => None,
        }
    }

    fn check_init(self: Pin<&mut Self>) -> bool {
        match &*self {
            Self::Init => match self.project_replace(Self::Empty) {
                ReplyStreamStateProjReplace::Init => true,
                _ => unreachable!(),
            },
            _ => false,
        }
    }
}
