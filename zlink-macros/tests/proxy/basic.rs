#[tokio::test]
async fn proxy_no_in_or_out_params() {
    use serde::{Deserialize, Serialize};
    use zlink::{proxy, test_utils::mock_socket::MockSocket, Connection};

    #[proxy("org.example.Basic")]
    trait BasicProxy {
        async fn ping(&mut self) -> zlink::Result<Result<(), BasicError>>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "error")]
    enum BasicError {
        NotFound,
        InvalidKey,
    }

    // Test that methods returning `()` should accept empty {} response and empty parameters.
    let responses = [r#"{}"#, r#"{parameters: {}}"#];
    let socket = MockSocket::new(&responses);

    let mut conn = Connection::new(socket);

    conn.ping().await.unwrap().unwrap();
}

#[tokio::test]
async fn proxy_oneway_method() {
    use zlink::{proxy, test_utils::mock_socket::MockSocket, Connection};

    #[proxy("org.example.Basic")]
    trait BasicProxy {
        #[zlink(oneway)]
        async fn notify(&mut self, message: String) -> zlink::Result<()>;
    }

    // Oneway methods don't expect a response, so we provide an empty response list
    let responses: &[&str] = &[];
    let socket = MockSocket::new(responses);

    let mut conn = Connection::new(socket);

    // This should send the message but not wait for a response
    conn.notify("Hello World".to_string()).await.unwrap();
}
