use serde::{Deserialize, Serialize};
use zlink::proxy;

#[derive(Debug, Serialize, Deserialize)]
struct Error;

#[test]
fn generic_compiles() {
    #[proxy("org.example.Generic")]
    #[allow(dead_code)]
    trait GenericProxy<T: Send + 'static> {
        async fn process<U: Serialize>(&mut self, data: U) -> zlink::Result<Result<String, Error>>;
    }
}

#[test]
fn where_clause_compiles() {
    #[proxy("org.example.WhereClause")]
    #[allow(dead_code)]
    trait WhereProxy<T>
    where
        T: Send + Sync + 'static,
    {
        async fn get(&mut self) -> zlink::Result<Result<String, Error>>;
    }
}
