#[test]
fn lifetimes_compile() {
    use serde::{Deserialize, Serialize};
    use zlink::proxy;

    #[proxy("org.example.Lifetimes")]
    #[allow(dead_code)]
    trait LifetimeProxy {
        async fn process<'a>(
            &mut self,
            data: &'a str,
        ) -> zlink::Result<Result<Response<'a>, Error>>;
        async fn with_lifetime<'b>(
            &mut self,
            input: &'b str,
            count: i32,
        ) -> zlink::Result<Result<Vec<&'b str>, Error>>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Response<'a> {
        data: &'a str,
        length: usize,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Error;
}
