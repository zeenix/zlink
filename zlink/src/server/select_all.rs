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
pub(super) struct SelectAll<'f, Fut> {
    futures: Vec<Pin<&'f mut Fut>, { super::MAX_CONNECTIONS }>,
}

impl<'f, Fut> SelectAll<'f, Fut>
where
    Fut: Future,
{
    /// Create a new `SelectAll`.
    pub(super) fn new() -> Self {
        SelectAll {
            futures: Vec::new(),
        }
    }

    /// Add a future to the `SelectAll`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the future is not moved/invalidated while it is in the
    /// `SelectAll`. The use case here is the Future impls created for `async fn` methods that are
    /// in reality `Unpin` but the compiler assumes they're `!Unpin`.
    pub(super) unsafe fn push_unchecked(&mut self, fut: &'f mut Fut) -> crate::Result<()> {
        self.futures
            .push(Pin::new_unchecked(fut))
            .map_err(|_| crate::Error::BufferOverflow)
    }
}

impl<'f, Fut> SelectAll<'f, Fut>
where
    Fut: Future + Unpin,
{
    /// Add a future to the `SelectAll`.
    pub(super) fn push(&mut self, fut: &'f mut Fut) -> crate::Result<()> {
        self.futures
            .push(Pin::new(fut))
            .map_err(|_| crate::Error::BufferOverflow)
    }
}

impl<Fut, Out> core::future::Future for SelectAll<'_, Fut>
where
    Fut: Future<Output = Out>,
{
    type Output = (usize, Out);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        for (i, fut) in (self.futures).iter_mut().enumerate() {
            if let Poll::Ready(item) = fut.as_mut().poll(cx) {
                return Poll::Ready((i, item));
            }
        }
        Poll::Pending
    }
}
