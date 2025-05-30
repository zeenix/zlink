//! Chain method calls.

mod reply_stream;

use crate::{connection::Socket, reply, Call, Connection, Result};
use core::fmt::Debug;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};

use reply_stream::ReplyStream;

/// A chain of method calls that will be sent together.
///
/// Each call must have the same method, reply, and error types for homogeneity. Use
/// [`Connection::chain_call`] to create a new chain, extend it with [`Chain::append`] and send the
/// the entire chain using [`Chain::send`].
///
/// With `std` feature enabled, this supports unlimited calls. Otherwise it is limited by how many
/// calls can fit in our fixed-sized buffer.
///
/// Oneway calls (where `Call::oneway() == Some(true)`) do not expect replies and are handled
/// automatically by the chain.
#[derive(Debug)]
pub struct Chain<'c, S: Socket, Method, Params, ReplyError> {
    pub(super) connection: &'c mut Connection<S>,
    pub(super) call_count: usize,
    pub(super) reply_count: usize,
    _phantom: core::marker::PhantomData<(Method, Params, ReplyError)>,
}

impl<'c, S, Method, Params, ReplyError> Chain<'c, S, Method, Params, ReplyError>
where
    S: Socket,
    Method: Serialize + Debug,
    Params: Deserialize<'c> + Debug,
    ReplyError: Deserialize<'c> + Debug,
{
    /// Create a new chain with the first call.
    pub(super) fn new(connection: &'c mut Connection<S>, call: &Call<Method>) -> Result<Self> {
        connection.write.enqueue_call(call)?;
        let reply_count = if call.oneway() == Some(true) { 0 } else { 1 };
        Ok(Chain {
            connection,
            call_count: 1,
            reply_count,
            _phantom: core::marker::PhantomData,
        })
    }

    /// Append another method call to the chain.
    ///
    /// The call will be enqueued but not sent until [`Chain::send`] is called. Note that one way
    /// calls (where `Call::oneway() == Some(true)`) do not receive replies.
    ///
    /// Calls with `more == Some(true)` will stream multiple replies until a reply with
    /// `continues != Some(true)` is received.
    pub fn append(mut self, call: &Call<Method>) -> Result<Self> {
        self.connection.write.enqueue_call(call)?;
        if call.oneway() != Some(true) {
            self.reply_count += 1;
        };
        self.call_count += 1;
        Ok(self)
    }

    /// Send all enqueued calls and return a replies stream.
    ///
    /// This will flush all enqueued calls in a single write operation and then return a stream
    /// that allows reading the replies.
    pub async fn send(
        self,
    ) -> Result<impl Stream<Item = Result<reply::Result<Params, ReplyError>>> + 'c>
    where
        Params: 'c,
        ReplyError: 'c,
    {
        // Flush all enqueued calls.
        self.connection.write.flush().await?;

        Ok(ReplyStream::new(
            self.connection.read_mut(),
            |conn| conn.receive_reply::<Params, ReplyError>(),
            self.reply_count,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        connection::socket::{ReadHalf, Socket, WriteHalf},
        Call,
    };
    use futures_util::pin_mut;
    use mayheap::Vec;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct GetUser {
        id: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct User {
        id: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ApiError {
        code: i32,
    }

    // Types for heterogeneous calls test
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "method")]
    enum HeterogeneousMethods {
        GetUser { id: u32 },
        GetPost { post_id: u32 },
        DeleteUser { user_id: u32 },
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Post {
        id: u32,
        title: mayheap::String<32>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct DeleteResult {
        success: bool,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum HeterogeneousResponses {
        Post(Post),
        User(User),
        DeleteResult(DeleteResult),
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct PostError {
        message: mayheap::String<64>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct DeleteError {
        reason: mayheap::String<64>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum HeterogeneousErrors {
        UserError(ApiError),
        PostError(PostError),
        DeleteError(DeleteError),
    }

    // Mock socket implementation for testing.
    #[derive(Debug)]
    struct MockSocket {
        read_data: Vec<u8, 1024>,
        read_pos: usize,
    }

    impl MockSocket {
        fn new(responses: &[&str]) -> Self {
            let mut data = Vec::new();

            for response in responses {
                data.extend_from_slice(response.as_bytes()).unwrap();
                data.push(b'\0').unwrap();
            }
            // Add an extra null byte to mark end of all messages
            data.push(b'\0').unwrap();

            Self {
                read_data: data,
                read_pos: 0,
            }
        }
    }

    impl Socket for MockSocket {
        type ReadHalf = MockReadHalf;
        type WriteHalf = MockWriteHalf;

        fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
            (
                MockReadHalf {
                    data: self.read_data,
                    pos: self.read_pos,
                },
                MockWriteHalf {
                    written: Vec::new(),
                },
            )
        }
    }

    #[derive(Debug)]
    struct MockReadHalf {
        data: Vec<u8, 1024>,
        pos: usize,
    }

    impl ReadHalf for MockReadHalf {
        async fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
            let remaining = self.data.len().saturating_sub(self.pos);
            if remaining == 0 {
                return Ok(0);
            }

            let to_read = remaining.min(buf.len());
            buf[..to_read].copy_from_slice(&self.data[self.pos..self.pos + to_read]);
            self.pos += to_read;
            Ok(to_read)
        }
    }

    #[derive(Debug)]
    struct MockWriteHalf {
        written: Vec<u8, 1024>,
    }

    impl WriteHalf for MockWriteHalf {
        async fn write(&mut self, buf: &[u8]) -> crate::Result<()> {
            self.written.extend_from_slice(buf).unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn homogeneous_calls() -> crate::Result<()> {
        let responses = [r#"{"parameters":{"id":1}}"#, r#"{"parameters":{"id":2}}"#];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let call1 = Call::new(GetUser { id: 1 });
        let call2 = Call::new(GetUser { id: 2 });

        let replies = conn
            .chain_call::<GetUser, User, ApiError>(&call1)?
            .append(&call2)?
            .send()
            .await?;

        use futures_util::stream::StreamExt;
        pin_mut!(replies);

        let user1 = replies.next().await.unwrap()?.unwrap();
        assert_eq!(user1.parameters().unwrap().id, 1);

        let user2 = replies.next().await.unwrap()?.unwrap();
        assert_eq!(user2.parameters().unwrap().id, 2);

        // No more replies should be available.
        let no_reply = replies.next().await;
        assert!(no_reply.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn oneway_calls_no_reply() -> crate::Result<()> {
        // Only the first call expects a reply; the second is oneway.
        let responses = [r#"{"parameters":{"id":1}}"#];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let get_user = Call::new(GetUser { id: 1 });
        let oneway_call = Call::new(GetUser { id: 2 }).set_oneway(Some(true));

        let replies = conn
            .chain_call::<GetUser, User, ApiError>(&get_user)?
            .append(&oneway_call)?
            .send()
            .await?;

        use futures_util::stream::StreamExt;
        pin_mut!(replies);

        let user = replies.next().await.unwrap()?.unwrap();
        assert_eq!(user.parameters().unwrap().id, 1);

        // No more replies should be available.
        let no_reply = replies.next().await;
        assert!(no_reply.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn more_calls_with_streaming() -> crate::Result<()> {
        let responses = [
            r#"{"parameters":{"id":1},"continues":true}"#,
            r#"{"parameters":{"id":2},"continues":true}"#,
            r#"{"parameters":{"id":3},"continues":false}"#,
            r#"{"parameters":{"id":4}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let more_call = Call::new(GetUser { id: 1 }).set_more(Some(true));
        let regular_call = Call::new(GetUser { id: 2 });

        let replies = conn
            .chain_call::<GetUser, User, ApiError>(&more_call)?
            .append(&regular_call)?
            .send()
            .await?;

        use futures_util::stream::StreamExt;
        pin_mut!(replies);

        // First call - streaming replies
        let user1 = replies.next().await.unwrap()?.unwrap();
        assert_eq!(user1.parameters().unwrap().id, 1);
        assert_eq!(user1.continues(), Some(true));

        let user2 = replies.next().await.unwrap()?.unwrap();
        assert_eq!(user2.parameters().unwrap().id, 2);
        assert_eq!(user2.continues(), Some(true));

        let user3 = replies.next().await.unwrap()?.unwrap();
        assert_eq!(user3.parameters().unwrap().id, 3);
        assert_eq!(user3.continues(), Some(false));

        // Second call - single reply
        let user4 = replies.next().await.unwrap()?.unwrap();
        assert_eq!(user4.parameters().unwrap().id, 4);
        assert_eq!(user4.continues(), None);

        // No more replies should be available.
        let no_reply = replies.next().await;
        assert!(no_reply.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn stream_interface_works() -> crate::Result<()> {
        use futures_util::stream::StreamExt;

        let responses = [
            r#"{"parameters":{"id":1}}"#,
            r#"{"parameters":{"id":2}}"#,
            r#"{"parameters":{"id":3}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let call1 = Call::new(GetUser { id: 1 });
        let call2 = Call::new(GetUser { id: 2 });
        let call3 = Call::new(GetUser { id: 3 });

        let replies = conn
            .chain_call::<GetUser, User, ApiError>(&call1)?
            .append(&call2)?
            .append(&call3)?
            .send()
            .await?;

        // Use Stream's collect method to gather all results
        pin_mut!(replies);
        let results: mayheap::Vec<_, 16> = replies.collect().await;
        assert_eq!(results.len(), 3);

        // Verify all results are successful
        for (i, result) in results.into_iter().enumerate() {
            let user = result?.unwrap();
            assert_eq!(user.parameters().unwrap().id, (i + 1) as u32);
        }

        Ok(())
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn heterogeneous_calls() -> crate::Result<()> {
        let responses = [
            r#"{"parameters":{"id":1}}"#,
            r#"{"parameters":{"id":123,"title":"Test Post"}}"#,
            r#"{"parameters":{"success":true}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let get_user_call = Call::new(HeterogeneousMethods::GetUser { id: 1 });
        let get_post_call = Call::new(HeterogeneousMethods::GetPost { post_id: 123 });
        let delete_user_call = Call::new(HeterogeneousMethods::DeleteUser { user_id: 456 });

        let replies = conn
            .chain_call::<HeterogeneousMethods, HeterogeneousResponses, HeterogeneousErrors>(
                &get_user_call,
            )?
            .append(&get_post_call)?
            .append(&delete_user_call)?
            .send()
            .await?;

        use futures_util::stream::StreamExt;
        pin_mut!(replies);

        // First response: User
        let user_response = replies.next().await.unwrap()?.unwrap();
        if let HeterogeneousResponses::User(user) = user_response.parameters().unwrap() {
            assert_eq!(user.id, 1);
        } else {
            panic!("Expected User response");
        }

        // Second response: Post
        let post_response = replies.next().await.unwrap()?.unwrap();
        if let HeterogeneousResponses::Post(post) = post_response.parameters().unwrap() {
            assert_eq!(post.id, 123);
            assert_eq!(post.title, "Test Post");
        } else {
            panic!("Expected Post response");
        }

        // Third response: DeleteResult
        let delete_response = replies.next().await.unwrap()?.unwrap();
        if let HeterogeneousResponses::DeleteResult(result) = delete_response.parameters().unwrap()
        {
            assert_eq!(result.success, true);
        } else {
            panic!("Expected DeleteResult response");
        }

        // No more replies should be available.
        let no_reply = replies.next().await;
        assert!(no_reply.is_none());
        Ok(())
    }
}
