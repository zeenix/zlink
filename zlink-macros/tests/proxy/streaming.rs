#[test]
fn streaming_compiles() {
    use futures_util::stream::Stream;
    use serde::{Deserialize, Serialize};
    use zlink::proxy;

    #[proxy("org.example.Stream")]
    #[allow(dead_code)]
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

    #[derive(Debug, Serialize, Deserialize)]
    struct Error;
}
