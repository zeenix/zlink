#[test]
fn rename_compiles() {
    use serde::{Deserialize, Serialize};
    use zlink::proxy;

    #[proxy("org.example.Rename")]
    #[allow(dead_code)]
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
}
