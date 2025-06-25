//! Enum type definition for Varlink IDL.

use core::fmt;

use serde::Serialize;

use super::super::List;

/// An enum type definition in Varlink IDL (enum-like with named variants).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum<'a> {
    /// The name of the enum type.
    name: &'a str,
    /// The variants of the enum type.
    variants: List<'a, &'a str>,
}

impl<'a> Enum<'a> {
    /// Creates a new enum type with the given name and borrowed variants.
    pub const fn new(name: &'a str, variants: &'a [&'a &'a str]) -> Self {
        Self {
            name,
            variants: List::Borrowed(variants),
        }
    }

    /// Creates a new enum type with the given name and owned variants.
    #[cfg(feature = "std")]
    pub fn new_owned(name: &'a str, variants: Vec<&'a str>) -> Self {
        Self {
            name,
            variants: List::Owned(variants),
        }
    }

    /// Returns the name of the enum type.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns an iterator over the variants of the enum type.
    pub fn variants(&self) -> impl Iterator<Item = &&'a str> {
        self.variants.iter()
    }
}

impl<'a> fmt::Display for Enum<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "type {} (", self.name)?;
        let mut first = true;
        for variant in self.variants.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{}", variant)?;
        }
        write!(f, ")")
    }
}

impl<'a> Serialize for Enum<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}
