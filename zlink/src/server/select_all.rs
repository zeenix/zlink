use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use mayheap::Vec;

/// A future that reads from multiple futures and returns the first one that is ready.
///
/// This is very similar to [`futures_util::future::SelectAll`] but much simpler and doesn't
/// (necessarily) allocate.
pub(super) struct SelectAll<Fut> {
    futures: Vec<Fut, { super::MAX_CONNECTIONS }>,
}

impl<Fut> SelectAll<Fut> {
    /// Create a new `SelectAll`.
    pub(super) fn new() -> Self {
        SelectAll {
            futures: Vec::new(),
        }
    }

    /// Add a future to the `SelectAll`.
    pub(super) fn push(&mut self, fut: Fut) -> crate::Result<()> {
        self.futures
            .push(fut)
            .map_err(|_| crate::Error::BufferOverflow)
    }
}

impl<Fut, Out> core::future::Future for SelectAll<Fut>
where
    Fut: Future<Output = Out> + Unpin,
{
    type Output = (usize, Out);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        for (i, fut) in (self.futures).iter_mut().enumerate() {
            if let Poll::Ready(item) = Pin::new(fut).poll(cx) {
                return Poll::Ready((i, item));
            }
        }
        Poll::Pending
    }
}
