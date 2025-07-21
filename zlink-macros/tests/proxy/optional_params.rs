#[test]
fn optional_params_compiles() {
    use serde::{Deserialize, Serialize};
    use zlink::proxy;

    #[proxy("org.example.Optional")]
    #[allow(dead_code)]
    trait OptionalProxy {
        async fn with_optionals(
            &mut self,
            required: &str,
            optional_string: Option<&str>,
            optional_number: Option<i32>,
            optional_bool: Option<bool>,
        ) -> zlink::Result<Result<String, Error>>;

        async fn mixed_optionals(
            &mut self,
            first: &str,
            opt1: Option<String>,
            second: i32,
            opt2: std::option::Option<bool>,
            third: &str,
            opt3: core::option::Option<Vec<String>>,
        ) -> zlink::Result<Result<(), Error>>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Error;
}
