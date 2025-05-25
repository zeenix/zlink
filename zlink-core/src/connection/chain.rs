//! Chain method calls.

use crate::{
    connection::Socket,
    reply::{self, Reply},
    Call, Connection, Result,
};
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

/// A chain of method calls that will be sent together.
///
/// Each call can have different method and reply types. Use [`Connection::chain_call`] to create a
/// new chain, extend it with [`Chain::append`] and then send the entire chain using
/// [`Chain::send`].
///
/// With `std` feature enabled, this supports unlimited calls. Otherwise it is limited by how many
/// calls can fit in our fixed-sized buffer.
///
/// Note that one way calls (where `Call::oneway() == Some(true)`) do not receive replies.
#[derive(Debug)]
pub struct Chain<'a, S: Socket> {
    pub(super) connection: &'a mut Connection<S>,
    pub(super) call_count: usize,
    pub(super) reply_count: usize,
}

impl<'a, S> Chain<'a, S>
where
    S: Socket,
{
    /// Append another method call to the chain.
    ///
    /// The call will be enqueued but not sent until [`Chain::send`] is called. Note that one way
    /// calls (where `Call::oneway() == Some(true)`) do not receive replies.
    ///
    /// Calls with `more == Some(true)` will stream multiple replies until a reply with
    /// `continues != Some(true)` is received.
    pub fn append<Method>(self, call: &Call<Method>) -> Result<Self>
    where
        Method: Serialize + Debug,
    {
        self.connection.write.enqueue_call(call)?;
        let reply_count = if call.oneway() == Some(true) {
            self.reply_count
        } else {
            self.reply_count + 1
        };
        Ok(Chain {
            connection: self.connection,
            call_count: self.call_count + 1,
            reply_count,
        })
    }

    /// Send all enqueued calls and return a replies accessor.
    ///
    /// This will flush all enqueued calls in a single write operation and then return a [`Replies`]
    /// that allows reading the replies.
    pub async fn send(self) -> Result<Replies<'a, S>> {
        // Flush all enqueued calls.
        self.connection.write.flush().await?;

        Ok(Replies {
            connection: self.connection,
            call_count: self.reply_count,
            current_index: 0,
        })
    }
}

/// The results of a chain of method calls.
#[derive(Debug)]
pub struct Replies<'a, S: Socket> {
    connection: &'a mut Connection<S>,
    call_count: usize,
    current_index: usize,
}

impl<'a, S: Socket> Replies<'a, S> {
    /// Get the number of replies expected.
    pub fn len(&self) -> usize {
        self.call_count
    }

    /// Check if there are no replies.
    pub fn is_empty(&self) -> bool {
        self.call_count == 0
    }

    /// Get the next reply with explicit type specification.
    ///
    /// Reads and parses the next reply. For calls with `more == Some(true)`, this will return
    /// multiple replies from the same call until a reply with `continues != Some(true)` is
    /// received.
    pub async fn next<Params, ReplyError>(
        &mut self,
    ) -> Result<Option<reply::Result<Params, ReplyError>>>
    where
        Params: for<'r> Deserialize<'r> + Debug,
        ReplyError: for<'r> Deserialize<'r> + Debug,
    {
        if self.current_index >= self.call_count {
            return Ok(None);
        }

        // Read the next reply directly from connection buffer
        let buffer = self.connection.read.read_message_bytes().await?;

        // Parse directly from connection's read buffer
        let result = match from_slice::<ReplyError>(buffer) {
            Ok(e) => Ok(Err(e)),
            Err(_) => from_slice::<Reply<Params>>(buffer).map(Ok),
        };

        // Only increment current_index if this is the last reply for this call
        // (i.e., continues is not Some(true))
        match &result {
            Ok(Ok(reply)) if reply.continues() != Some(true) => {
                self.current_index += 1;
            }
            Ok(Ok(_)) => {
                // Streaming reply, don't increment index yet
            }
            Ok(Err(_)) | Err(_) => {
                // For errors, always increment since there won't be more replies
                self.current_index += 1;
            }
        }

        result.map(Some)
    }

    /// Process all remaining replies with a closure.
    ///
    /// This is a convenience method that reads all remaining replies and applies
    /// the provided closure to each one. Useful for simple iteration patterns.
    ///
    /// # Example
    /// ```no_run
    /// # use zlink_core::connection::chain::Replies;
    /// # use zlink_core::{Connection, reply};
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Debug, Deserialize)]
    /// # struct User { name: String }
    /// # #[derive(Debug, Deserialize)]
    /// # struct ApiError { code: i32 }
    /// # async fn example() -> zlink_core::Result<()> {
    /// # let mut replies: Replies<'_, zlink_core::connection::socket::impl_for_doc::Socket> = todo!();
    /// replies.for_each::<User, ApiError, _>(|reply| {
    ///     match reply {
    ///         Ok(user) => println!("User: {}", user.parameters().unwrap().name),
    ///         Err(error) => println!("Error: {}", error.code),
    ///     }
    ///     Ok(())
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn for_each<Params, ReplyError, F>(&mut self, mut f: F) -> Result<()>
    where
        Params: for<'r> Deserialize<'r> + Debug,
        ReplyError: for<'r> Deserialize<'r> + Debug,
        F: FnMut(reply::Result<Params, ReplyError>) -> Result<()>,
    {
        while let Some(reply) = self.next::<Params, ReplyError>().await? {
            f(reply)?;
        }
        Ok(())
    }
}

fn from_slice<'a, T>(buffer: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    #[cfg(feature = "std")]
    {
        serde_json::from_slice::<T>(buffer).map_err(Into::into)
    }

    #[cfg(not(feature = "std"))]
    {
        serde_json_core::from_slice::<T>(buffer)
            .map_err(Into::into)
            .map(|(e, _)| e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        connection::socket::{ReadHalf, Socket, WriteHalf},
        Call,
    };
    use mayheap::{String, Vec};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct GetUser {
        id: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct GetProject {
        id: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct User {
        id: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Project {
        id: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ApiError {
        code: i32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ProjectError {
        error: String<128>,
        parameters: ProjectErrorParams,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ProjectErrorParams {
        code: i32,
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
    async fn heterogeneous_two_calls() {
        let responses = [r#"{"parameters":{"id":1}}"#, r#"{"parameters":{"id":2}}"#];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let get_user = Call::new(GetUser { id: 1 });
        let get_project = Call::new(GetProject { id: 2 });

        let mut replies = conn
            .chain_call(&get_user)
            .unwrap()
            .append(&get_project)
            .unwrap()
            .send()
            .await
            .unwrap();

        assert_eq!(replies.len(), 2);

        // Test sequential access with explicit types
        let user_reply: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let project_reply: reply::Result<Project, ProjectError> =
            replies.next().await.unwrap().unwrap();

        assert!(user_reply.is_ok());
        assert!(project_reply.is_ok());

        let user = user_reply.unwrap();
        assert_eq!(user.parameters().unwrap().id, 1);
        let project = project_reply.unwrap();
        assert_eq!(project.parameters().unwrap().id, 2);
    }

    #[tokio::test]
    async fn heterogeneous_with_error() {
        let responses = [
            r#"{"parameters":{"id":1}}"#,
            r#"{"error":"org.example.ProjectError","parameters":{"code":-1}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let get_user = Call::new(GetUser { id: 1 });
        let get_project = Call::new(GetProject { id: 99 }); // This will error

        let mut replies = conn
            .chain_call(&get_user)
            .unwrap()
            .append(&get_project)
            .unwrap()
            .send()
            .await
            .unwrap();

        assert_eq!(replies.len(), 2);

        let user_reply: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let project_reply: reply::Result<Project, ProjectError> =
            replies.next().await.unwrap().unwrap();

        // Verify mixed success/error handling
        assert!(user_reply.is_ok());
        assert!(project_reply.is_err());

        let user = user_reply.unwrap();
        assert_eq!(user.parameters().unwrap().id, 1);
        let error = project_reply.unwrap_err();
        assert_eq!(error.error, "org.example.ProjectError");
        assert_eq!(error.parameters.code, -1);
    }

    #[tokio::test]
    async fn single_call_chain() {
        let responses = [r#"{"parameters":{"id":1}}"#];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let get_user = Call::new(GetUser { id: 1 });

        let mut replies = conn.chain_call(&get_user).unwrap().send().await.unwrap();

        assert_eq!(replies.len(), 1);

        let user_reply: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user = user_reply.unwrap();
        assert_eq!(user.parameters().unwrap().id, 1);
    }

    #[tokio::test]
    async fn many_calls_chain() {
        let responses = [
            r#"{"parameters":{"id":1}}"#,
            r#"{"parameters":{"id":2}}"#,
            r#"{"parameters":{"id":3}}"#,
            r#"{"parameters":{"id":4}}"#,
            r#"{"parameters":{"id":5}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let mut chain = conn.chain_call(&Call::new(GetUser { id: 1 })).unwrap();
        for i in 2..=5 {
            chain = chain.append(&Call::new(GetUser { id: i })).unwrap();
        }

        let mut replies = chain.send().await.unwrap();
        assert_eq!(replies.len(), 5);

        for i in 1..=5 {
            let user_reply: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
            let user = user_reply.unwrap();
            assert_eq!(user.parameters().unwrap().id, i as u32);
        }
    }

    #[tokio::test]
    async fn for_each_convenience_method() {
        let responses = [
            r#"{"parameters":{"id":1}}"#,
            r#"{"parameters":{"id":2}}"#,
            r#"{"parameters":{"id":3}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let mut chain = conn.chain_call(&Call::new(GetUser { id: 1 })).unwrap();
        chain = chain.append(&Call::new(GetUser { id: 2 })).unwrap();
        chain = chain.append(&Call::new(GetUser { id: 3 })).unwrap();

        let mut replies = chain.send().await.unwrap();

        let mut count = 0;
        replies
            .for_each::<User, ApiError, _>(|reply| {
                count += 1;
                let user = reply.unwrap();
                assert_eq!(user.parameters().unwrap().id, count);
                Ok(())
            })
            .await
            .unwrap();

        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn oneway_calls_no_reply() {
        // Only the first call expects a reply; the second is one way.
        let responses = [r#"{"parameters":{"id":1}}"#];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let get_user = Call::new(GetUser { id: 1 });
        let oneway_call = Call::new(GetUser { id: 2 }).set_oneway(Some(true));

        let mut replies = conn
            .chain_call(&get_user)
            .unwrap()
            .append(&oneway_call)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Should only expect 1 reply, not 2.
        assert_eq!(replies.len(), 1);

        let user_reply: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user = user_reply.unwrap();
        assert_eq!(user.parameters().unwrap().id, 1);

        // No more replies should be available.
        let no_reply = replies.next::<User, ApiError>().await.unwrap();
        assert!(no_reply.is_none());
    }

    #[tokio::test]
    async fn mixed_oneway_and_regular_calls() {
        // Three calls: regular, one way, regular - only 2 replies expected.
        let responses = [r#"{"parameters":{"id":1}}"#, r#"{"parameters":{"id":3}}"#];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let call1 = Call::new(GetUser { id: 1 });
        let oneway_call = Call::new(GetUser { id: 2 }).set_oneway(Some(true));
        let call3 = Call::new(GetUser { id: 3 });

        let mut replies = conn
            .chain_call(&call1)
            .unwrap()
            .append(&oneway_call)
            .unwrap()
            .append(&call3)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Should only expect 2 replies (call1 and call3).
        assert_eq!(replies.len(), 2);

        let reply1: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user1 = reply1.unwrap();
        assert_eq!(user1.parameters().unwrap().id, 1);

        let reply3: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user3 = reply3.unwrap();
        assert_eq!(user3.parameters().unwrap().id, 3);

        // No more replies should be available.
        let no_reply = replies.next::<User, ApiError>().await.unwrap();
        assert!(no_reply.is_none());
    }

    #[tokio::test]
    async fn all_oneway_calls() {
        // All calls are one way - no replies expected.
        let responses = [];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let oneway1 = Call::new(GetUser { id: 1 }).set_oneway(Some(true));
        let oneway2 = Call::new(GetUser { id: 2 }).set_oneway(Some(true));

        let mut replies = conn
            .chain_call(&oneway1)
            .unwrap()
            .append(&oneway2)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Should expect 0 replies.
        assert_eq!(replies.len(), 0);
        assert!(replies.is_empty());

        // No replies should be available.
        let no_reply = replies.next::<User, ApiError>().await.unwrap();
        assert!(no_reply.is_none());
    }

    #[tokio::test]
    async fn more_calls_with_streaming() {
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

        let mut replies = conn
            .chain_call(&more_call)
            .unwrap()
            .append(&regular_call)
            .unwrap()
            .send()
            .await
            .unwrap();

        assert_eq!(replies.len(), 2); // 2 calls, even though first call streams multiple replies

        // First call - streaming replies
        let reply1: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user1 = reply1.unwrap();
        assert_eq!(user1.parameters().unwrap().id, 1);
        assert_eq!(user1.continues(), Some(true));

        let reply2: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user2 = reply2.unwrap();
        assert_eq!(user2.parameters().unwrap().id, 2);
        assert_eq!(user2.continues(), Some(true));

        let reply3: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user3 = reply3.unwrap();
        assert_eq!(user3.parameters().unwrap().id, 3);
        assert_eq!(user3.continues(), Some(false));

        // Second call - single reply
        let reply4: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user4 = reply4.unwrap();
        assert_eq!(user4.parameters().unwrap().id, 4);
        assert_eq!(user4.continues(), None);

        // No more replies should be available.
        let no_reply = replies.next::<User, ApiError>().await.unwrap();
        assert!(no_reply.is_none());
    }

    #[tokio::test]
    async fn more_calls_with_error_midstream() {
        let responses = [
            r#"{"parameters":{"id":1},"continues":true}"#,
            r#"{"code":-1}"#,
            r#"{"parameters":{"id":3}}"#,
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let more_call = Call::new(GetUser { id: 1 }).set_more(Some(true));
        let regular_call = Call::new(GetUser { id: 2 });

        let mut replies = conn
            .chain_call(&more_call)
            .unwrap()
            .append(&regular_call)
            .unwrap()
            .send()
            .await
            .unwrap();

        assert_eq!(replies.len(), 2); // 2 calls

        // First streaming reply
        let reply1: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user1 = reply1.unwrap();
        assert_eq!(user1.parameters().unwrap().id, 1);
        assert_eq!(user1.continues(), Some(true));

        // Error reply - should increment current_index since error terminates the stream
        let error_reply: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let error = error_reply.unwrap_err();
        assert_eq!(error.code, -1);

        // Second call - single reply
        let reply3: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user3 = reply3.unwrap();
        assert_eq!(user3.parameters().unwrap().id, 3);
        assert_eq!(user3.continues(), None);

        // No more replies should be available.
        let no_reply = replies.next::<User, ApiError>().await.unwrap();
        assert!(no_reply.is_none());
    }

    #[tokio::test]
    async fn multiple_more_calls_in_sequence() {
        let responses = [
            // First more call - 2 streaming replies
            r#"{"parameters":{"id":1},"continues":true}"#,
            r#"{"parameters":{"id":2},"continues":false}"#,
            // Second more call - 3 streaming replies
            r#"{"parameters":{"id":10},"continues":true}"#,
            r#"{"parameters":{"id":20},"continues":true}"#,
            r#"{"parameters":{"id":30}}"#, // No continues field means false
        ];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let more_call1 = Call::new(GetUser { id: 1 }).set_more(Some(true));
        let more_call2 = Call::new(GetUser { id: 2 }).set_more(Some(true));

        let mut replies = conn
            .chain_call(&more_call1)
            .unwrap()
            .append(&more_call2)
            .unwrap()
            .send()
            .await
            .unwrap();

        assert_eq!(replies.len(), 2); // 2 calls

        // First call - streaming replies
        let reply1: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user1 = reply1.unwrap();
        assert_eq!(user1.parameters().unwrap().id, 1);
        assert_eq!(user1.continues(), Some(true));

        let reply2: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user2 = reply2.unwrap();
        assert_eq!(user2.parameters().unwrap().id, 2);
        assert_eq!(user2.continues(), Some(false));

        // Second call - streaming replies
        let reply3: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user3 = reply3.unwrap();
        assert_eq!(user3.parameters().unwrap().id, 10);
        assert_eq!(user3.continues(), Some(true));

        let reply4: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user4 = reply4.unwrap();
        assert_eq!(user4.parameters().unwrap().id, 20);
        assert_eq!(user4.continues(), Some(true));

        let reply5: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user5 = reply5.unwrap();
        assert_eq!(user5.parameters().unwrap().id, 30);
        assert_eq!(user5.continues(), None); // No continues field

        // No more replies should be available.
        let no_reply = replies.next::<User, ApiError>().await.unwrap();
        assert!(no_reply.is_none());
    }

    #[tokio::test]
    async fn more_false_calls_are_supported() {
        let responses = [r#"{"parameters":{"id":1}}"#, r#"{"parameters":{"id":2}}"#];
        let socket = MockSocket::new(&responses);
        let mut conn = Connection::new(socket);

        let call1 = Call::new(GetUser { id: 1 }).set_more(Some(false));
        let call2 = Call::new(GetUser { id: 2 }).set_more(Some(false));

        let mut replies = conn
            .chain_call(&call1)
            .unwrap()
            .append(&call2)
            .unwrap()
            .send()
            .await
            .unwrap();

        assert_eq!(replies.len(), 2);

        let reply1: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user1 = reply1.unwrap();
        assert_eq!(user1.parameters().unwrap().id, 1);

        let reply2: reply::Result<User, ApiError> = replies.next().await.unwrap().unwrap();
        let user2 = reply2.unwrap();
        assert_eq!(user2.parameters().unwrap().id, 2);
    }
}
