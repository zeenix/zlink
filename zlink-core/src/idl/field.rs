//! Custom type, field and parameter definitions for Varlink IDL.

use core::fmt;

use super::{Comment, List, Type, TypeRef};

/// A field in a custom type or method parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<'a> {
    /// The name of the field.
    name: &'a str,
    /// The type of the field.
    ty: TypeRef<'a>,
    /// Comments associated with this field.
    comments: List<'a, Comment<'a>>,
}

/// Type alias for method parameters, which have the same structure as fields.
pub type Parameter<'a> = Field<'a>;

impl<'a> Field<'a> {
    /// Creates a new field with the given name, borrowed type, and comments.
    pub const fn new(name: &'a str, ty: &'a Type<'a>, comments: &'a [&'a Comment<'a>]) -> Self {
        Self {
            name,
            ty: TypeRef::new(ty),
            comments: List::Borrowed(comments),
        }
    }

    /// Same as `new` but takes `ty` by value.
    /// Creates a new field with the given name, owned type, and comments.
    #[cfg(feature = "std")]
    pub fn new_owned(name: &'a str, ty: Type<'a>, comments: Vec<Comment<'a>>) -> Self {
        Self {
            name,
            ty: TypeRef::new_owned(ty),
            comments: List::from(comments),
        }
    }

    /// Returns the name of the field.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the type of the field.
    pub fn ty(&self) -> &Type<'a> {
        self.ty.inner()
    }

    /// Returns the comments associated with this field.
    pub fn comments(&self) -> impl Iterator<Item = &Comment<'a>> {
        self.comments.iter()
    }
}

impl<'a> fmt::Display for Field<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::Type;

    #[test]
    fn field_creation() {
        let field = Field::new("age", &Type::Int, &[]);
        assert_eq!(field.name(), "age");
        assert_eq!(field.ty(), &Type::Int);
    }

    #[test]
    fn parameter_alias() {
        let param: Parameter<'_> = Field::new("input", &Type::String, &[]);
        assert_eq!(param.name(), "input");
        assert_eq!(param.ty(), &Type::String);
    }
}
