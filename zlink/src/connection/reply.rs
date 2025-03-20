use serde::{Deserialize, Serialize};

/// A successful method call reply.
#[derive(Debug, Serialize, Deserialize)]
pub struct Reply<Params> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parameters: Option<Params>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) continues: Option<bool>,
}

impl<Params> Reply<Params> {
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
