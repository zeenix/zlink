//! Varlink field names.

use crate::parse;

/// Whether `name` is a valid Varlink field name.
///
/// Struct fields, method parameters and enum variants all follow this rule.
///
/// # Examples
///
/// ```
/// use zlink_names::is_valid_field_name;
///
/// assert!(is_valid_field_name("user_name"));
/// assert!(!is_valid_field_name("user-name"));
/// ```
pub fn is_valid_field_name(name: &str) -> bool {
    parse::parses_fully(parse::field_name, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_names() {
        let valid = ["a", "Z", "user_name", "user0", "a_0_b", "type", "foo123"];
        for name in valid {
            assert!(is_valid_field_name(name), "`{name}` must be valid");
        }

        let invalid = [
            "",
            "_foo",
            "0foo",
            "user-name",
            "user name",
            "user.name",
            "user!",
            "üser",
            "user\u{e9}",
            "x__",
            "foo__bar",
        ];
        for name in invalid {
            assert!(!is_valid_field_name(name), "`{name}` must be invalid");
        }
    }
}
