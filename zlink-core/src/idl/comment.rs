//! Comment definitions for Varlink interfaces.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

/// A comment in a Varlink interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment<'a> {
    /// The comment text content (without the leading #).
    content: &'a str,
}

impl<'a> Comment<'a> {
    /// Creates a new comment with the given text content (without # prefix).
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }

    /// Returns the comment text content.
    pub fn content(&self) -> &'a str {
        self.content
    }

    /// Returns the comment text content.
    pub fn text(&self) -> &'a str {
        self.content
    }
}

impl<'a> fmt::Display for Comment<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "# {}", self.content)
    }
}

impl<'a> Serialize for Comment<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "idl-parse")]
impl<'de, 'a> Deserialize<'de> for Comment<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Ok(Comment::new(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_creation() {
        let comment = Comment::new("This is a comment");
        assert_eq!(comment.content(), "This is a comment");
        assert_eq!(comment.text(), "This is a comment");
    }

    #[test]
    fn comment_without_hash() {
        let comment = Comment::new("This is not a proper comment");
        assert_eq!(comment.content(), "This is not a proper comment");
        assert_eq!(comment.text(), "This is not a proper comment");
    }

    #[test]
    fn comment_display() {
        let comment = Comment::new("A enum field allowing to gracefully get metadata");
        use core::fmt::Write;
        let mut buf = mayheap::String::<128>::new();
        write!(buf, "{}", comment).unwrap();
        assert_eq!(
            buf.as_str(),
            "# A enum field allowing to gracefully get metadata"
        );
    }

    #[test]
    fn comment_serialization() {
        let comment = Comment::new("This is a test comment");
        #[cfg(feature = "std")]
        let json = serde_json::to_string(&comment).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 64];
            let len = serde_json_core::to_slice(&comment, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 64>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<64>::from_utf8(vec).unwrap()
        };
        assert_eq!(json, "\"# This is a test comment\"");
    }
}
