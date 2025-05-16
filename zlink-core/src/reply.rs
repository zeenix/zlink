//! Method reply API.

use serde::{Deserialize, Serialize};

/// A successful method call reply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply<Params> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parameters: Option<Params>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) continues: Option<bool>,
}

impl<Params> Reply<Params> {
    /// Create a new reply.
    pub fn new(parameters: Option<Params>) -> Self {
        Self {
            parameters,
            continues: None,
        }
    }

    /// Set the continues flag.
    pub fn set_continues(mut self, continues: Option<bool>) -> Self {
        self.continues = continues;
        self
    }

    /// The parameters of the reply.
    pub fn parameters(&self) -> Option<&Params> {
        self.parameters.as_ref()
    }

    /// Convert the reply into its parameters.
    pub fn into_parameters(self) -> Option<Params> {
        self.parameters
    }

    /// If there are more replies to come.
    pub fn continues(&self) -> Option<bool> {
        self.continues
    }
}

impl<Params> From<Params> for Reply<Params> {
    fn from(parameters: Params) -> Self {
        Self::new(Some(parameters))
    }
}

/// A reply result.
pub type Result<Params, Error> = core::result::Result<Reply<Params>, Error>;
