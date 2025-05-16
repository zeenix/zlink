use core::fmt::Debug;
use mayheap::String;
use serde::Serialize;

use crate::Result;

#[derive(Debug, Serialize)]
pub(super) struct Method<Params> {
    method: String<MAX_METHOD_NAME_LEN>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<Params>,
}

impl<Params> Method<Params>
where
    Params: Serialize + Debug,
{
    pub(super) fn new(name: &str, parameters: Option<Params>) -> Result<Self> {
        let mut method_name: String<MAX_METHOD_NAME_LEN> = String::new();
        method_name.push('.')?;
        method_name.push_str(name)?;

        Ok(Method {
            method: method_name,
            parameters,
        })
    }
}

const MAX_METHOD_NAME_LEN: usize = 32;
