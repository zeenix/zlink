#[test]
fn basic_compiles() {
    use serde::{Deserialize, Serialize};
    use zlink::proxy;

    #[proxy("org.example.Basic")]
    #[allow(dead_code)]
    trait BasicProxy {
        async fn get_value(&mut self, key: &str) -> zlink::Result<Result<String, BasicError>>;
        async fn set_value(
            &mut self,
            key: &str,
            value: &str,
        ) -> zlink::Result<Result<(), BasicError>>;
        async fn ping(&mut self) -> zlink::Result<Result<(), BasicError>>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "error")]
    enum BasicError {
        NotFound,
        InvalidKey,
    }
}
