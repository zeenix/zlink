//! Enum type definition for Varlink IDL.

use core::fmt;

use super::List;

/// An enum type definition in Varlink IDL (enum-like with named variants).
#[derive(Debug, Clone, Eq)]
pub struct CustomEnum<'a> {
    /// The name of the enum type.
    name: &'a str,
    /// The variants of the enum type.
    variants: List<'a, &'a str>,
    /// The comments associated with this enum type.
    comments: List<'a, super::Comment<'a>>,
}

impl<'a> CustomEnum<'a> {
    /// Creates a new enum type with the given name, borrowed variants, and comments.
    pub const fn new(
        name: &'a str,
        variants: &'a [&'a &'a str],
        comments: &'a [&'a super::Comment<'a>],
    ) -> Self {
        Self {
            name,
            variants: List::Borrowed(variants),
            comments: List::Borrowed(comments),
        }
    }

    /// Creates a new enum type with the given name, owned variants, and comments.
    #[cfg(feature = "std")]
    pub fn new_owned(
        name: &'a str,
        variants: Vec<&'a str>,
        comments: Vec<super::Comment<'a>>,
    ) -> Self {
        Self {
            name,
            variants: List::from(variants),
            comments: List::from(comments),
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

    /// Returns an iterator over the comments associated with this enum type.
    pub fn comments(&self) -> impl Iterator<Item = &super::Comment<'a>> {
        self.comments.iter()
    }
}

impl<'a> fmt::Display for CustomEnum<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Comments first
        for comment in self.comments.iter() {
            writeln!(f, "{comment}")?;
        }
        write!(f, "type {} (", self.name)?;
        let mut first = true;
        for variant in self.variants.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{variant}")?;
        }
        write!(f, ")")
    }
}

impl<'a> PartialEq for CustomEnum<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.variants == other.variants
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::Comment;
    use core::fmt::Write;

    #[test]
    fn display_with_comments() {
        let comment1 = Comment::new("Status enumeration");
        let comment2 = Comment::new("Represents current state");
        let comments = [&comment1, &comment2];

        let var1 = "active";
        let var2 = "inactive";
        let var3 = "pending";
        let variants = [&var1, &var2, &var3];

        let custom_enum = CustomEnum::new("Status", &variants, &comments);
        let mut displayed = mayheap::String::<128>::new();
        write!(&mut displayed, "{}", custom_enum).unwrap();
        assert_eq!(
            displayed,
            "# Status enumeration\n# Represents current state\ntype Status (active, inactive, pending)"
        );
    }
}
