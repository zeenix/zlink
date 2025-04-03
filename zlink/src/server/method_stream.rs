use core::{
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};

use pin_project_lite::pin_project;
use serde::Deserialize;

use crate::connection::{socket::ReadHalf, Call, ReadConnection};

pin_project! {
    /// A method call stream based on [`ReadConnection`].
    ///
    /// This would be useful for external use as well but we keep it internal because
    /// [`ReadConnection::receive_call`] gives us anonymous futures so we have to keep the future
    /// type generic here. This can be solved with Boxing but we want to avoid all allocations in
    /// the core library.
    pub(super) struct MethodStream<'c, Read: ReadHalf, F, Fut> {
        #[pin]
        state: MethodStreamState<Fut>,
        conn: &'c mut ReadConnection<Read>,
        func: F,
    }
}

impl<'c, Read, F, Fut, Method> MethodStream<'c, Read, F, Fut>
where
    Read: ReadHalf,
    F: FnMut(&'c mut ReadConnection<Read>) -> Fut,
    Fut: Future<Output = crate::Result<Call<Method>>>,
    Method: Deserialize<'c>,
{
    pub(super) fn new(conn: &'c mut ReadConnection<Read>, func: F) -> Self {
        MethodStream {
            state: MethodStreamState::Init,
            conn,
            func,
        }
    }
}

impl<'c, Read, F, Fut, Method> futures_util::stream::Stream for MethodStream<'c, Read, F, Fut>
where
    Read: ReadHalf,
    F: FnMut(&'c mut ReadConnection<Read>) -> Fut,
    Fut: Future<Output = crate::Result<Call<Method>>>,
    Method: Deserialize<'c>,
{
    type Item = crate::Result<Call<Method>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if this.state.as_mut().check_init() {
            let conn = unsafe { &mut *((*this.conn) as *mut _) };
            this.state.set(MethodStreamState::Future {
                future: (this.func)(conn),
            });
        }

        let item = match this.state.as_mut().project_future() {
            Some(fut) => ready!(fut.poll(cx)),
            None => panic!("Unfold must not be polled after it returned `Poll::Ready(None)`"),
        };

        this.state.set(MethodStreamState::Init);
        Poll::Ready(Some(item))
    }
}

pin_project! {
    /// State for `MethodStream`.
    ///
    /// Based on the [`futures::stream::unfold`] implementation.
    #[project = MethodStreamStateProj]
    #[project_replace = MethodStreamStateProjReplace]
    #[derive(Debug)]
    enum MethodStreamState<R> {
        Init,
        Future {
            #[pin]
            future: R,
        },
        Empty,
    }
}

impl<R> MethodStreamState<R> {
    fn project_future(self: Pin<&mut Self>) -> Option<Pin<&mut R>> {
        match self.project() {
            MethodStreamStateProj::Future { future } => Some(future),
            _ => None,
        }
    }

    fn check_init(self: Pin<&mut Self>) -> bool {
        match &*self {
            Self::Init => match self.project_replace(Self::Empty) {
                MethodStreamStateProjReplace::Init => true,
                _ => unreachable!(),
            },
            _ => false,
        }
    }
}
