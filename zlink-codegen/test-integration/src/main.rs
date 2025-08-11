// Include the generated code from the build script.
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Test types from test.idl.
    let _person = Person {
        name: "Alice".to_string(),
        age: 30,
        email: Some("alice@example.com".to_string()),
    };

    // Test error types.
    let _error = ExampleError::NotFound {
        message: "User not found".to_string(),
    };

    // Test that proxy traits exist and compile.
    fn assert_example_proxy<T: ExampleProxy>() {}
    fn assert_calc_proxy<T: CalcProxy>() {}
    fn assert_storage_proxy<T: StorageProxy>() {}

    // Test creating and using Document type from storage.idl.
    let _doc = Document {
        id: "doc123".to_string(),
        title: "Test Document".to_string(),
        content: "This is test content".to_string(),
        tags: vec!["test".to_string(), "example".to_string()],
    };

    println!("All generated types and proxies compile correctly!");
    Ok(())
}