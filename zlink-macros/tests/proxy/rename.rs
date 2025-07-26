#[tokio::test]
async fn rename_test() {
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use zlink::{proxy, test_utils::mock_socket::MockSocket, Connection};

    #[proxy("org.example.Rename")]
    trait RenameProxy {
        #[zlink(rename = "GetData")]
        async fn get_data(&mut self) -> zlink::Result<Result<String, Error>>;

        #[zlink(rename = "SetValue")]
        async fn update_value(&mut self, value: i32) -> zlink::Result<Result<(), Error>>;

        // Test snake_case to PascalCase conversion
        async fn snake_case_method(&mut self) -> zlink::Result<Result<(), Error>>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Error;

    // Test get_data with renamed method
    let responses = json!({"parameters": "test data"}).to_string();
    let socket = MockSocket::new(&[&responses]);
    let mut conn = Connection::new(socket);

    let result = conn.get_data().await.unwrap().unwrap();
    assert_eq!(result, "test data");

    // Verify the renamed method name is used
    let bytes_written = conn.write().write_half().written_data();
    let written: serde_json::Value =
        serde_json::from_slice(&bytes_written[..bytes_written.len() - 1]).unwrap();
    assert_eq!(written["method"], "org.example.Rename.GetData");

    // Test update_value
    let responses = json!({}).to_string();
    let socket = MockSocket::new(&[&responses]);
    let mut conn = Connection::new(socket);

    conn.update_value(42).await.unwrap().unwrap();

    // Test snake_case_method (should be converted to PascalCase)
    let responses = json!({}).to_string();
    let socket = MockSocket::new(&[&responses]);
    let mut conn = Connection::new(socket);

    conn.snake_case_method().await.unwrap().unwrap();

    // Verify snake_case was converted to PascalCase
    let bytes_written = conn.write().write_half().written_data();
    let written: serde_json::Value =
        serde_json::from_slice(&bytes_written[..bytes_written.len() - 1]).unwrap();
    assert_eq!(written["method"], "org.example.Rename.SnakeCaseMethod");
}
