use futures_util::TryStreamExt;

#[tokio::test]
async fn streaming_test() {
    use futures_util::stream::Stream;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use zlink::{proxy, test_utils::mock_socket::MockSocket, Connection};

    #[proxy("org.example.Stream")]
    trait StreamProxy {
        async fn get_single(&mut self) -> zlink::Result<Result<String, Error>>;

        #[zlink(more)]
        async fn get_stream(
            &mut self,
        ) -> zlink::Result<impl Stream<Item = zlink::Result<Result<String, Error>>>>;

        #[zlink(rename = "CustomStream", more)]
        async fn custom_stream(
            &mut self,
            count: i32,
        ) -> zlink::Result<impl Stream<Item = zlink::Result<Result<Item, Error>>>>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Item {
        id: u32,
        name: String,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Error;

    // Test get_single
    let responses = json!({"parameters": "single result"}).to_string();
    let socket = MockSocket::new(&[&responses]);
    let mut conn = Connection::new(socket);

    let result = conn.get_single().await.unwrap().unwrap();
    assert_eq!(result, "single result");

    // Test streaming method
    let responses = [
        json!({"continues": true, "parameters": "stream item 1"}).to_string(),
        json!({"continues": true, "parameters": "stream item 2"}).to_string(),
        json!({"continues": false, "parameters": "stream item 3"}).to_string(),
    ];
    let socket = MockSocket::new(&responses.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let mut conn = Connection::new(socket);

    let stream = conn.get_stream().await.unwrap();
    futures_util::pin_mut!(stream);
    let items = stream
        .try_collect::<Vec<Result<String, Error>>>()
        .await
        .unwrap();
    assert_eq!(
        items,
        vec![
            Ok("stream item 1".to_string()),
            Ok("stream item 2".to_string()),
            Ok("stream item 3".to_string())
        ]
    );

    // Test custom_stream method
    let responses = [
        json!({"continues": true, "parameters": {"id": 1, "name": "Item 1"}}).to_string(),
        json!({"continues": true, "parameters": {"id": 2, "name": "Item 2"}}).to_string(),
        json!({"continues": false, "parameters": {"id": 3, "name": "Item 3"}}).to_string(),
    ];
    let socket = MockSocket::new(&responses.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let mut conn = Connection::new(socket);

    let stream = conn.custom_stream(3).await.unwrap();
    futures_util::pin_mut!(stream);
    let items = stream
        .try_collect::<Vec<Result<Item, Error>>>()
        .await
        .unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].as_ref().unwrap().id, 1);
    assert_eq!(items[0].as_ref().unwrap().name, "Item 1");
    assert_eq!(items[1].as_ref().unwrap().id, 2);
    assert_eq!(items[2].as_ref().unwrap().id, 3);
}
