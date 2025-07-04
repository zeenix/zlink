//! Object type definition for Varlink IDL.

use core::fmt;

use super::{Field, List};

/// An object type definition in Varlink IDL (struct-like with named fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomObject<'a> {
    /// The name of the object type.
    name: &'a str,
    /// The fields of the object type.
    fields: List<'a, Field<'a>>,
    /// The comments associated with this object type.
    comments: List<'a, super::Comment<'a>>,
}

impl<'a> CustomObject<'a> {
    /// Creates a new object type with the given name, borrowed fields, and comments.
    pub const fn new(
        name: &'a str,
        fields: &'a [&'a Field<'a>],
        comments: &'a [&'a super::Comment<'a>],
    ) -> Self {
        Self {
            name,
            fields: List::Borrowed(fields),
            comments: List::Borrowed(comments),
        }
    }

    /// Creates a new object type with the given name, owned fields, and comments.
    #[cfg(feature = "std")]
    pub fn new_owned(
        name: &'a str,
        fields: Vec<Field<'a>>,
        comments: Vec<super::Comment<'a>>,
    ) -> Self {
        Self {
            name,
            fields: List::from(fields),
            comments: List::from(comments),
        }
    }

    /// Returns the name of the object type.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns an iterator over the fields of the object type.
    pub fn fields(&self) -> impl Iterator<Item = &Field<'a>> {
        self.fields.iter()
    }

    /// Returns an iterator over the comments associated with this object type.
    pub fn comments(&self) -> impl Iterator<Item = &super::Comment<'a>> {
        self.comments.iter()
    }
}

impl<'a> fmt::Display for CustomObject<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type {} (", self.name)?;
        let mut first = true;
        for field in self.fields.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{field}")?;
        }
        write!(f, ")")
    }
}
