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
#[derive(Debug)]
pub struct Chain<'a, S: Socket> {
    pub(super) connection: &'a mut Connection<S>,
    pub(super) call_count: usize,
}

impl<'a, S> Chain<'a, S>
where
    S: Socket,
{
    /// Append another method call to the chain.
    ///
    /// The call will be enqueued but not sent until [`Chain::send`] is called.
    pub fn append<Method>(self, call: &Call<Method>) -> Result<Self>
    where
        Method: Serialize + Debug,
    {
        self.connection.write.enqueue_call(call)?;
        Ok(Chain {
            connection: self.connection,
            call_count: self.call_count + 1,
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
            call_count: self.call_count,
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
    /// Reads and parses the next reply.
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
        self.current_index += 1;

        // Parse directly from connection's read buffer
        let result = match from_slice::<ReplyError>(buffer) {
            Ok(e) => Ok(Err(e)),
            Err(_) => from_slice::<Reply<Params>>(buffer).map(Ok),
        };

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

        if let Ok(user) = user_reply {
            assert_eq!(user.parameters().unwrap().id, 1);
        }
        if let Ok(project) = project_reply {
            assert_eq!(project.parameters().unwrap().id, 2);
        }
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

        if let Ok(user) = user_reply {
            assert_eq!(user.parameters().unwrap().id, 1);
        }
        if let Err(error) = project_reply {
            assert_eq!(error.error, "org.example.ProjectError");
            assert_eq!(error.parameters.code, -1);
        }
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
        assert!(user_reply.is_ok());
        if let Ok(user) = user_reply {
            assert_eq!(user.parameters().unwrap().id, 1);
        }
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
            assert!(user_reply.is_ok());
            if let Ok(user) = user_reply {
                assert_eq!(user.parameters().unwrap().id, i as u32);
            }
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
                assert!(reply.is_ok());
                if let Ok(user) = reply {
                    assert_eq!(user.parameters().unwrap().id, count);
                }
                Ok(())
            })
            .await
            .unwrap();

        assert_eq!(count, 3);
    }
}
