use core::fmt::Debug;

use serde::{Deserialize, Serialize};

/// A method call.
#[derive(Debug, Serialize, Deserialize)]
pub struct Call<M> {
    #[serde(flatten)]
    pub(super) method: M,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) oneway: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) more: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) upgrade: Option<bool>,
}

impl<M> Call<M> {
    /// The method call name and parameters.
    pub fn method(&self) -> &M {
        &self.method
    }

    /// If the method call doesn't want a reply.
    pub fn oneway(&self) -> Option<bool> {
        self.oneway
    }

    /// If the method call is requesting more replies.
    pub fn more(&self) -> Option<bool> {
        self.more
    }

    /// If the method call is requesting an upgrade to a different protocol.
    pub fn upgrade(&self) -> Option<bool> {
        self.upgrade
    }
}
