use super::Error;

/// Introspection trait for method reply errors type.
pub trait ReplyErrors {
    /// The list of possible errors this type can return.
    const REPLY_ERRORS: &'static [&'static Error<'static>];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_errors() {
        // Test with a type that implements ReplyErrors.
        struct MyType;

        const MY_ERROR: Error<'static> = Error::new("MyError", &[]);

        impl ReplyErrors for MyType {
            const REPLY_ERRORS: &'static [&'static Error<'static>] = &[&MY_ERROR];
        }

        assert_eq!(MyType::REPLY_ERRORS.len(), 1);
        assert_eq!(MyType::REPLY_ERRORS[0].name(), "MyError");
    }
}
