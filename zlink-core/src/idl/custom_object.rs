//! Object type definition for Varlink IDL.

use core::fmt;

use serde::Serialize;

use super::{Field, List};

/// An object type definition in Varlink IDL (struct-like with named fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomObject<'a> {
    /// The name of the object type.
    name: &'a str,
    /// The fields of the object type.
    fields: List<'a, Field<'a>>,
}

impl<'a> CustomObject<'a> {
    /// Creates a new object type with the given name and borrowed fields.
    pub const fn new(name: &'a str, fields: &'a [&'a Field<'a>]) -> Self {
        Self {
            name,
            fields: List::Borrowed(fields),
        }
    }

    /// Creates a new object type with the given name and owned fields.
    #[cfg(feature = "std")]
    pub fn new_owned(name: &'a str, fields: Vec<Field<'a>>) -> Self {
        Self {
            name,
            fields: List::Owned(fields),
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
            write!(f, "{}", field)?;
        }
        write!(f, ")")
    }
}

impl<'a> Serialize for CustomObject<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}
