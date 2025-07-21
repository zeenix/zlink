#[test]
fn complex_lifetimes_compile() {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use zlink::proxy;

    #[proxy("org.example.Complex")]
    #[allow(dead_code)]
    trait ComplexProxy {
        async fn process_array(
            &mut self,
            items: &[String],
        ) -> zlink::Result<Result<Vec<Item>, Error>>;

        async fn process_nested(
            &mut self,
            data: HashMap<String, Vec<Option<Item>>>,
        ) -> zlink::Result<Result<Option<Vec<String>>, Error>>;

        async fn with_tuples(
            &mut self,
            pairs: Vec<(String, i32)>,
        ) -> zlink::Result<Result<(bool, Option<String>), Error>>;

        async fn generic_result<'a>(
            &mut self,
            input: &'a str,
        ) -> zlink::Result<Result<Response<'a>, CustomError<'a>>>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Item {
        id: u32,
        name: String,
        tags: Vec<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Response<'a> {
        message: &'a str,
        success: bool,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Error;

    #[derive(Debug, Serialize, Deserialize)]
    struct CustomError<'a> {
        code: u32,
        message: &'a str,
    }
}
