use serde::{Deserialize, Serialize};
use zlink::proxy;

#[derive(Debug, Serialize, Deserialize)]
struct Error;

#[test]
fn generic_compiles() {
    #[proxy("org.example.Generic")]
    #[allow(dead_code)]
    trait GenericProxy<T: Serialize + std::fmt::Debug> {
        async fn process(&mut self, data: T) -> zlink::Result<Result<&str, Error>>;
        async fn process2<U: Serialize + std::fmt::Debug>(
            &mut self,
            data: U,
        ) -> zlink::Result<Result<&str, Error>>;
    }
}

#[test]
fn where_clause_compiles() {
    #[proxy("org.example.WhereClause")]
    #[allow(dead_code)]
    trait WhereProxy<T>
    where
        T: Serialize + std::fmt::Debug,
    {
        async fn get(&mut self, value: T) -> zlink::Result<Result<String, Error>>;
        async fn get2<U>(&mut self, value: U) -> zlink::Result<Result<String, Error>>
        where
            U: Serialize + std::fmt::Debug;
    }
}
