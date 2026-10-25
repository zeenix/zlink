//! Varlink type names.

use crate::parse;

/// Whether `name` is a valid Varlink type name.
///
/// Method and error names follow this rule too.
///
/// # Examples
///
/// ```
/// use zlink_names::is_valid_type_name;
///
/// assert!(is_valid_type_name("FileNotFound"));
/// assert!(!is_valid_type_name("fileNotFound"));
/// ```
pub fn is_valid_type_name(name: &str) -> bool {
    parse::parses_fully(parse::type_name, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::is_valid_field_name;

    #[test]
    fn type_names() {
        let valid = ["A", "Foo", "FileNotFound", "Foo0", "X"];
        for name in valid {
            assert!(is_valid_type_name(name), "`{name}` must be valid");
        }

        let invalid = [
            "", "foo", "_Foo", "0Foo", "Foo_bar", "Foo-bar", "Foo bar", "Foo.Bar", "Foo!", "Über",
        ];
        for name in invalid {
            assert!(!is_valid_type_name(name), "`{name}` must be invalid");
        }
    }

    /// Type names are the stricter rule: every one of them is also a valid field name.
    #[test]
    fn type_names_are_field_names() {
        for name in ["A", "Foo", "FileNotFound", "Foo0"] {
            assert!(is_valid_field_name(name), "`{name}` must be valid");
        }
    }
}
