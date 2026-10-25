//! Varlink interface names.

use crate::parse;

/// Whether `name` is a valid Varlink interface name.
///
/// Interface names are reverse-domain notation: dot-separated segments, at least one dot.
///
/// # Examples
///
/// ```
/// use zlink_names::is_valid_interface_name;
///
/// assert!(is_valid_interface_name("org.example.Foo"));
/// assert!(!is_valid_interface_name("Foo"));
/// ```
pub fn is_valid_interface_name(name: &str) -> bool {
    parse::parses_fully(parse::interface_name, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_names() {
        let valid = ["org.example.Foo", "a.b", "org.varlink.service", "a1.b-2.c3"];
        for name in valid {
            assert!(is_valid_interface_name(name), "`{name}` must be valid");
        }

        let invalid = [
            "",
            "Foo",          // No dot.
            "org.",         // Trailing dot, empty segment.
            ".org",         // Leading dot.
            "org..example", // Empty segment.
            "org.example.", // Trailing dot.
            "1org.example", // First segment must start with a letter.
            "org.example.Foo bar",
            "org.example.Foö",
        ];
        for name in invalid {
            assert!(!is_valid_interface_name(name), "`{name}` must be invalid");
        }
    }
}
