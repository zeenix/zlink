use serde::Deserialize;

use super::{Info, InterfaceDescription};

/// Union type for all possible reply parameters from the `org.varlink.service` interface.
///
/// This enum represents all possible replies from the varlink service interface methods.
/// Each proxy implementation should provide a similar enum for all its replies.
///
/// The `#[serde(untagged)]` attribute allows serde to automatically deserialize
/// the correct variant based on the structure of the JSON data.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Reply<'a> {
    /// Reply from GetInfo method.
    #[serde(borrow)]
    Info(Info<'a>),
    /// Reply from GetInterfaceDescription method.
    /// Note: InterfaceDescription only supports 'static lifetime for deserialization.
    InterfaceDescription(InterfaceDescription<'static>),
}
