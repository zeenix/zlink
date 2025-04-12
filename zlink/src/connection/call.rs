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
    /// Create a new method call.
    pub fn new(method: M) -> Self {
        Self {
            method,
            oneway: None,
            more: None,
            upgrade: None,
        }
    }

    /// Set the oneway flag.
    pub fn set_oneway(mut self, oneway: Option<bool>) -> Self {
        self.oneway = oneway;
        self
    }

    /// Set the more flag.
    pub fn set_more(mut self, more: Option<bool>) -> Self {
        self.more = more;
        self
    }

    /// Set the upgrade flag.
    pub fn set_upgrade(mut self, upgrade: Option<bool>) -> Self {
        self.upgrade = upgrade;
        self
    }

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

impl<M> From<M> for Call<M> {
    fn from(method: M) -> Self {
        Self::new(method)
    }
}
