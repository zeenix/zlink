//! Parsers for Varlink IDL using winnow.
//!
//! This module provides parsers for converting IDL strings into the corresponding
//! Rust types defined in the parent module. Uses byte-based parsing to avoid UTF-8 overhead.

use winnow::{
    ascii::multispace0,
    combinator::{alt, separated},
    error::{ErrMode, InputError, ParserError},
    token::{literal, take_while},
    ModalResult, Parser,
};

use super::{
    Comment, CustomEnum, CustomObject, CustomType, Error, Field, Interface, List, Member, Method,
    Parameter, Type, TypeRef,
};

#[cfg(feature = "std")]
use std::vec::Vec;

/// Parse whitespace and comments according to Varlink grammar.
/// The `_` production in Varlink grammar: whitespace / comment / eol_r
fn ws<'a>(input: &mut &'a [u8]) -> ModalResult<(), InputError<&'a [u8]>> {
    loop {
        let start_len = input.len();

        // Consume regular whitespace (spaces, tabs, etc.)
        multispace0::<_, InputError<&'a [u8]>>
            .parse_next(input)
            .ok();

        // Try to consume a comment: "#" [^\n\r\u{2028}\u{2029}]* eol_r
        if input.starts_with(b"#") {
            // Skip the '#'
            *input = &input[1..];

            // Consume everything until end of line
            while !input.is_empty() {
                match input[0] {
                    b'\n' | b'\r' => {
                        // Consume the end-of-line character(s)
                        if input.starts_with(b"\r\n") {
                            *input = &input[2..];
                        } else {
                            *input = &input[1..];
                        }
                        break;
                    }
                    _ => {
                        *input = &input[1..];
                    }
                }
            }
        }

        // If we didn't consume anything in this iteration, break
        if input.len() == start_len {
            break;
        }
    }
    Ok(())
}

/// Parse only whitespace (not comments) - used in interface parsing where comments are members.
fn whitespace_only<'a>(input: &mut &'a [u8]) -> ModalResult<(), InputError<&'a [u8]>> {
    multispace0::<_, InputError<&'a [u8]>>
        .parse_next(input)
        .ok();
    Ok(())
}

/// Convert bytes to str with input lifetime.
fn bytes_to_str(bytes: &[u8]) -> &str {
    // SAFETY: We only accept ASCII characters in our parsers
    core::str::from_utf8(bytes).unwrap()
}

/// Parse a field name: starts with letter, continues with alphanumeric and underscores.
fn field_name<'a>(input: &mut &'a [u8]) -> ModalResult<&'a str, InputError<&'a [u8]>> {
    let start = *input;
    let mut pos = 0;

    // First character must be alphabetic
    if pos >= input.len() || !input[pos].is_ascii_alphabetic() {
        return Err(ErrMode::Backtrack(ParserError::from_input(input)));
    }
    pos += 1;

    // Continue with alphanumeric and underscores
    while pos < input.len() && (input[pos].is_ascii_alphanumeric() || input[pos] == b'_') {
        pos += 1;
    }

    let name_bytes = &start[0..pos];
    *input = &input[pos..];
    Ok(bytes_to_str(name_bytes))
}

/// Parse a type name: starts with uppercase letter, continues with alphanumeric.
fn type_name<'a>(input: &mut &'a [u8]) -> ModalResult<&'a str, InputError<&'a [u8]>> {
    let start = *input;
    if input.is_empty() || !input[0].is_ascii_uppercase() {
        return Err(ErrMode::Backtrack(ParserError::from_input(input)));
    }

    let mut end = 1;
    while end < input.len() && input[end].is_ascii_alphanumeric() {
        end += 1;
    }

    let name_bytes = &start[0..end];
    *input = &input[end..];
    Ok(bytes_to_str(name_bytes))
}

/// Parse a primitive type.
fn primitive_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    alt((
        literal("bool").map(|_| Type::Bool),
        literal("int").map(|_| Type::Int),
        literal("float").map(|_| Type::Float),
        literal("string").map(|_| Type::String),
        literal("object").map(|_| Type::ForeignObject),
    ))
    .parse_next(input)
}

/// Parse a field in a struct or parameter list.
fn field<'a>(input: &mut &'a [u8]) -> ModalResult<Field<'a>, InputError<&'a [u8]>> {
    let comments = parse_preceding_comments(input)?;

    let name = field_name(input)?;
    ws(input)?;
    literal(":").parse_next(input)?;
    ws(input)?;
    let ty = varlink_type(input)?;
    Ok(Field::new_owned(name, ty, comments))
}

/// Parse an inline struct type: (field1: type1, field2: type2).
fn struct_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    literal("(").parse_next(input)?;
    ws(input)?;
    let fields: Vec<Field<'a>> = separated(0.., field, (ws, literal(","), ws)).parse_next(input)?;
    ws(input)?;
    literal(")").parse_next(input)?;
    Ok(Type::Object(List::from(fields)))
}

/// Parse an inline enum type: (variant1, variant2, variant3).
fn enum_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    literal("(").parse_next(input)?;
    ws(input)?;
    let variants: Vec<&str> =
        separated(0.., field_name, (ws, literal(","), ws)).parse_next(input)?;
    ws(input)?;
    literal(")").parse_next(input)?;
    Ok(Type::Enum(List::from(variants)))
}

/// Parse an inline type (struct or enum).
/// Determines if it's a struct by looking for ':' character.
fn inline_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    // Look ahead to see if this contains a colon (indicating struct)
    if let Some(pos) = input.iter().position(|&b| b == b')') {
        let content = &input[1..pos]; // Skip opening paren
        if content.contains(&b':') {
            struct_type(input)
        } else {
            enum_type(input)
        }
    } else {
        Err(ErrMode::Backtrack(ParserError::from_input(input)))
    }
}

/// Parse an element type (primitive, custom, or inline).
fn element_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    alt((primitive_type, type_name.map(Type::Custom), inline_type)).parse_next(input)
}

/// Parse an optional type: ?type.
fn optional_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    literal("?").parse_next(input)?;
    let inner = non_optional_type(input)?;
    Ok(Type::Optional(TypeRef::new_owned(inner)))
}

/// Parse any type except optional (to avoid recursion).
fn non_optional_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    alt((array_type, map_type, element_type)).parse_next(input)
}

/// Parse an array type: []type.
fn array_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    literal("[]").parse_next(input)?;
    let inner = varlink_type(input)?;
    Ok(Type::Array(TypeRef::new_owned(inner)))
}

/// Parse a map type: [string]type.
fn map_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    literal("[string]").parse_next(input)?;
    let inner = varlink_type(input)?;
    Ok(Type::Map(TypeRef::new_owned(inner)))
}

/// Parse any Varlink type.
fn varlink_type<'a>(input: &mut &'a [u8]) -> ModalResult<Type<'a>, InputError<&'a [u8]>> {
    alt((optional_type, array_type, map_type, element_type)).parse_next(input)
}

/// Parse an interface name: reverse domain notation like org.example.test.
fn interface_name<'a>(input: &mut &'a [u8]) -> ModalResult<&'a str, InputError<&'a [u8]>> {
    let start = *input;
    let mut pos = 0;

    // First segment: [A-Za-z]([-]*[A-Za-z0-9])*
    if pos >= input.len() || !input[pos].is_ascii_alphabetic() {
        return Err(ErrMode::Backtrack(ParserError::from_input(input)));
    }
    pos += 1;

    while pos < input.len() && (input[pos].is_ascii_alphanumeric() || input[pos] == b'-') {
        pos += 1;
    }

    let mut found_dot = false;
    // Subsequent segments: .[A-Za-z0-9]([-]*[A-Za-z0-9])*
    while pos < input.len() && input[pos] == b'.' {
        found_dot = true;
        pos += 1; // skip dot

        // Must have at least one alphanumeric after dot
        if pos >= input.len() || !input[pos].is_ascii_alphanumeric() {
            break;
        }
        pos += 1;

        // Continue with alphanumeric and dashes
        while pos < input.len() && (input[pos].is_ascii_alphanumeric() || input[pos] == b'-') {
            pos += 1;
        }
    }

    // Check for at least one dot
    if !found_dot {
        return Err(ErrMode::Backtrack(ParserError::from_input(input)));
    }

    let name_bytes = &start[0..pos];
    *input = &input[pos..];
    Ok(bytes_to_str(name_bytes))
}

/// Parse a parameter list: (param1: type1, param2: type2).
fn parameter_list<'a>(
    input: &mut &'a [u8],
) -> ModalResult<Vec<Parameter<'a>>, InputError<&'a [u8]>> {
    literal("(").parse_next(input)?;
    whitespace_only(input)?;

    let mut params = Vec::new();

    // Handle empty parameter list
    if literal::<_, _, InputError<&'a [u8]>>(")")
        .parse_next(input)
        .is_ok()
    {
        return Ok(params);
    }

    // Parse first parameter with any preceding comments
    loop {
        // Parse any preceding comments for this parameter
        let comments = parse_preceding_comments(input)?;

        // Parse the parameter itself (field name and type)
        let name = field_name(input)?;
        ws(input)?;
        literal(":").parse_next(input)?;
        ws(input)?;
        let ty = varlink_type(input)?;

        params.push(Parameter::new_owned(name, ty, comments));

        whitespace_only(input)?;

        // Check for comma (more parameters) or closing paren (end)
        if literal::<_, _, InputError<&'a [u8]>>(",")
            .parse_next(input)
            .is_ok()
        {
            whitespace_only(input)?;
            // Continue to next parameter
        } else if literal::<_, _, InputError<&'a [u8]>>(")")
            .parse_next(input)
            .is_ok()
        {
            // End of parameter list
            break;
        } else {
            return Err(ErrMode::Backtrack(ParserError::from_input(input)));
        }
    }

    Ok(params)
}

/// Parse a method definition: method Name(inputs) -> (outputs).
fn method_def<'a>(input: &mut &'a [u8]) -> ModalResult<Method<'a>, InputError<&'a [u8]>> {
    let comments = parse_preceding_comments(input)?;

    literal("method").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = type_name(input)?;
    ws(input)?;
    let input_params = parameter_list(input)?;
    ws(input)?;
    literal("->").parse_next(input)?;
    ws(input)?;
    let output_params = parameter_list(input)?;

    Ok(Method::new_owned(
        name,
        input_params,
        output_params,
        comments,
    ))
}

/// Parse an error definition: error Name (fields).
fn error_def<'a>(input: &mut &'a [u8]) -> ModalResult<Error<'a>, InputError<&'a [u8]>> {
    let comments = parse_preceding_comments(input)?;

    literal("error").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = type_name(input)?;
    ws(input)?;
    let params = parameter_list(input)?;

    Ok(Error::new_owned(name, params, comments))
}

/// Parse a type definition: type Name <definition>.
fn type_def<'a>(input: &mut &'a [u8]) -> ModalResult<CustomType<'a>, InputError<&'a [u8]>> {
    let comments = parse_preceding_comments(input)?;

    literal("type").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = type_name(input)?;
    ws(input)?;
    literal("(").parse_next(input)?;
    whitespace_only(input)?;

    let mut fields = Vec::new();
    let mut variants = Vec::new();
    let mut has_typed_fields = false;
    let mut has_untyped_fields = false;

    // Handle empty field list
    if literal::<_, _, InputError<&'a [u8]>>(")")
        .parse_next(input)
        .is_ok()
    {
        return Ok(CustomType::from(CustomObject::new_owned(
            name, fields, comments,
        )));
    }

    // Parse fields with any preceding comments
    loop {
        // Parse any preceding comments for this field
        let field_comments = parse_preceding_comments(input)?;

        // Parse the field itself
        let field_name = field_name(input)?;
        whitespace_only(input)?;

        // Try to parse the colon and type
        if literal::<_, _, InputError<&'a [u8]>>(":")
            .parse_next(input)
            .is_ok()
        {
            whitespace_only(input)?;
            let ty = varlink_type(input)?;
            fields.push(Field::new_owned(field_name, ty, field_comments));
            has_typed_fields = true;
        } else {
            // This is an enum-like field without type - collect as variant
            variants.push(field_name);
            has_untyped_fields = true;
        }

        whitespace_only(input)?;

        // Check for comma (more fields) or closing paren (end)
        if literal::<_, _, InputError<&'a [u8]>>(",")
            .parse_next(input)
            .is_ok()
        {
            whitespace_only(input)?;
            // Continue to next field
        } else if literal::<_, _, InputError<&'a [u8]>>(")")
            .parse_next(input)
            .is_ok()
        {
            // End of field list
            break;
        } else {
            return Err(ErrMode::Backtrack(ParserError::from_input(input)));
        }
    }

    // Error if we have both typed and untyped fields (mixed custom type)
    if has_typed_fields && has_untyped_fields {
        return Err(ErrMode::Backtrack(ParserError::from_input(input)));
    }

    // Decide whether to create an enum or object based on whether we saw typed fields
    if has_typed_fields {
        Ok(CustomType::from(CustomObject::new_owned(
            name, fields, comments,
        )))
    } else {
        // All fields were untyped, so this is an enum
        Ok(CustomType::from(CustomEnum::new_owned(
            name, variants, comments,
        )))
    }
}

/// Parse a member definition (type, method, or error).
/// Helper function to parse any preceding comments.
fn parse_preceding_comments<'a>(
    input: &mut &'a [u8],
) -> ModalResult<Vec<Comment<'a>>, InputError<&'a [u8]>> {
    let mut comments = Vec::new();
    while !input.is_empty() {
        let checkpoint = *input;
        whitespace_only(input)?;

        if input.is_empty() {
            break;
        }

        if let Ok(comment) = comment_def(input) {
            comments.push(comment);
            whitespace_only(input)?;
        } else {
            // Not a comment, restore position
            *input = checkpoint;
            break;
        }
    }
    Ok(comments)
}

fn comment_def<'a>(input: &mut &'a [u8]) -> ModalResult<Comment<'a>, InputError<&'a [u8]>> {
    literal("#").parse_next(input)?;

    // Skip all leading whitespace after #
    while !input.is_empty() && (input[0] == b' ' || input[0] == b'\t') {
        *input = &input[1..];
    }

    // Take until newline or end of input - this is the actual comment content
    let line_content = take_while(0.., |c: u8| c != b'\n').parse_next(input)?;
    let comment_text = bytes_to_str(line_content);

    Ok(Comment::new(comment_text))
}

fn member_def<'a>(input: &mut &'a [u8]) -> ModalResult<Member<'a>, InputError<&'a [u8]>> {
    alt((
        type_def.map(Member::Custom),
        method_def.map(Member::Method),
        error_def.map(Member::Error),
    ))
    .parse_next(input)
}

/// Parse an interface definition.
fn interface_def<'a>(input: &mut &'a [u8]) -> ModalResult<Interface<'a>, InputError<&'a [u8]>> {
    let comments = parse_preceding_comments(input)?;

    literal("interface").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = interface_name(input)?;
    whitespace_only(input)?;

    // Parse members separated by whitespace/newlines
    let mut members = Vec::new();

    while !input.is_empty() {
        whitespace_only(input)?;

        if input.is_empty() {
            break;
        }

        match member_def(input) {
            Ok(member) => {
                members.push(member);
            }
            Err(_) => break,
        }
    }

    Ok(Interface::new_owned(name, members, comments))
}

/// Parse an interface from a string.
pub(super) fn parse_interface(input: &str) -> Result<Interface<'_>, crate::Error> {
    parse_from_str(input, interface_def)
}

/// Helper function to parse from string using byte-based parsers.
fn parse_from_str<'a, T>(
    input: &'a str,
    parser: impl Fn(&mut &'a [u8]) -> ModalResult<T, InputError<&'a [u8]>>,
) -> Result<T, crate::Error> {
    let input_bytes = input.trim().as_bytes();
    if input_bytes.is_empty() {
        return Err(crate::Error::IdlParse("Input is empty".to_string()));
    }

    let mut input_mut = input_bytes;
    match parser(&mut input_mut) {
        Ok(result) => {
            let _ = ws(&mut input_mut);
            if input_mut.is_empty() {
                Ok(result)
            } else {
                Err(crate::Error::IdlParse(format!(
                    "Unexpected remaining input: {:?}",
                    core::str::from_utf8(input_mut).map_or("<invalid UTF-8>", |s| s)
                )))
            }
        }
        Err(err) => Err(crate::Error::IdlParse(format!("Parse error: {err}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_primitive_types() {
        assert_eq!(parse_type("bool").unwrap(), Type::Bool);
        assert_eq!(parse_type("int").unwrap(), Type::Int);
        assert_eq!(parse_type("float").unwrap(), Type::Float);
        assert_eq!(parse_type("string").unwrap(), Type::String);
        assert_eq!(parse_type("object").unwrap(), Type::ForeignObject);
    }

    #[test]
    fn test_parse_custom_types() {
        match parse_type("Person").unwrap() {
            Type::Custom(name) => {
                assert_eq!(name, "Person");
            }
            _ => panic!("Expected custom type"),
        }
    }

    #[test]
    fn test_parse_composite_types() {
        // Test optional type
        match parse_type("?int").unwrap() {
            Type::Optional(optional) => {
                assert_eq!(*optional, Type::Int);
            }
            _ => panic!("Expected optional type"),
        }

        // Test array type
        match parse_type("[]string").unwrap() {
            Type::Array(array) => {
                assert_eq!(*array, Type::String);
            }
            _ => panic!("Expected array type"),
        }

        // Test map type
        match parse_type("[string]bool").unwrap() {
            Type::Map(map) => {
                assert_eq!(*map, Type::Bool);
            }
            _ => panic!("Expected map type"),
        }
    }

    #[test]
    fn test_parse_nested_types() {
        // Test nested optional array
        match parse_type("?[]string").unwrap() {
            Type::Optional(optional) => match &*optional {
                Type::Array(array) => {
                    assert_eq!(*array, Type::String);
                }
                _ => panic!("Expected array inside optional"),
            },
            _ => panic!("Expected optional type"),
        }
    }

    #[test]
    fn test_parse_inline_enum() {
        match parse_type("(one, two, three)").unwrap() {
            Type::Enum(variants) => {
                let collected: Vec<_> = variants.iter().collect();
                assert_eq!(collected.len(), 3);
            }
            _ => panic!("Expected enum type"),
        }
    }

    #[test]
    fn test_parse_inline_struct() {
        match parse_type("(x: float, y: float)").unwrap() {
            Type::Object(fields) => {
                let collected: Vec<_> = fields.iter().collect();
                assert_eq!(collected.len(), 2);
                assert_eq!(collected[0].name(), "x");
                assert_eq!(collected[0].ty(), &Type::Float);
                assert_eq!(collected[1].name(), "y");
                assert_eq!(collected[1].ty(), &Type::Float);
            }
            _ => panic!("Expected struct type"),
        }
    }

    #[test]
    fn test_parse_whitespace() {
        // Test that whitespace is handled correctly
        assert_eq!(parse_type("  bool  ").unwrap(), Type::Bool);
        assert_eq!(parse_type("\tbool\n").unwrap(), Type::Bool);
    }

    #[test]
    fn test_parse_errors() {
        assert!(parse_type("").is_err());
        assert!(parse_type("invalid").is_err());
        assert!(parse_type("bool extra").is_err());
    }

    #[test]
    fn test_parse_interface_name() {
        let input = b"org.example.test";
        let mut input_mut = input.as_slice();
        let result = interface_name(&mut input_mut).unwrap();
        assert_eq!(result, "org.example.test");
        assert!(input_mut.is_empty());

        let input = b"com.example.foo.bar";
        let mut input_mut = input.as_slice();
        let result = interface_name(&mut input_mut).unwrap();
        assert_eq!(result, "com.example.foo.bar");
        assert!(input_mut.is_empty());

        // Invalid: no dot
        let mut input_mut = b"example".as_slice();
        assert!(interface_name(&mut input_mut).is_err());

        // Invalid: starts with number
        let mut input_mut = b"1example.test".as_slice();
        assert!(interface_name(&mut input_mut).is_err());
    }

    #[test]
    fn test_parse_method() {
        let input = "method GetInfo() -> (info: string)";
        let method = parse_method(input).unwrap();
        assert_eq!(method.name(), "GetInfo");
        assert_eq!(method.inputs().count(), 0);
        let mut outputs = method.outputs();
        assert_eq!(
            outputs.next().unwrap(),
            &Parameter::new("info", &Type::String, &[])
        );
        assert!(outputs.next().is_none());

        let input = "method Add(a: int, b: int) -> (sum: int)";
        let method = parse_method(input).unwrap();
        assert_eq!(method.name(), "Add");
        let mut inputs = method.inputs();
        assert_eq!(
            inputs.next().unwrap(),
            &Parameter::new("a", &Type::Int, &[])
        );
        assert_eq!(
            inputs.next().unwrap(),
            &Parameter::new("b", &Type::Int, &[])
        );
        assert!(inputs.next().is_none());
        let mut outputs = method.outputs();
        assert_eq!(
            outputs.next().unwrap(),
            &Parameter::new("sum", &Type::Int, &[])
        );
        assert!(outputs.next().is_none());
    }

    #[test]
    fn test_parse_error() {
        let input = "error NotFound(resource: string)";
        let error = parse_error(input).unwrap();
        assert_eq!(error.name(), "NotFound");
        assert_eq!(error.fields().count(), 1);
        let mut fields = error.fields();
        assert_eq!(
            fields.next().unwrap(),
            &Field::new("resource", &Type::String, &[])
        );
        assert!(fields.next().is_none());

        let input = "error InvalidInput()";
        let error = parse_error(input).unwrap();
        assert_eq!(error.name(), "InvalidInput");
        assert_eq!(error.fields().count(), 0);
    }

    #[test]
    fn test_parse_custom_type() {
        let input = "type Person (name: string, age: int)";
        let custom_type = parse_custom_type(input).unwrap();
        assert_eq!(custom_type.name(), "Person");
        let mut fields = custom_type.as_object().unwrap().fields();
        assert_eq!(
            fields.next().unwrap(),
            &Field::new("name", &Type::String, &[])
        );
        assert_eq!(fields.next().unwrap(), &Field::new("age", &Type::Int, &[]));
        assert!(fields.next().is_none());

        let input = "type Config (host: string, port: int, enabled: bool)";
        let custom_type = parse_custom_type(input).unwrap();
        assert_eq!(custom_type.name(), "Config");
        assert_eq!(custom_type.as_object().unwrap().fields().count(), 3);
        let mut fields = custom_type.as_object().unwrap().fields();
        assert_eq!(
            fields.next().unwrap(),
            &Field::new("host", &Type::String, &[])
        );
        assert_eq!(fields.next().unwrap(), &Field::new("port", &Type::Int, &[]));
        assert_eq!(
            fields.next().unwrap(),
            &Field::new("enabled", &Type::Bool, &[])
        );
        assert!(fields.next().is_none());

        // Enum-style type definitions are correctly parsed as enums
        let enum_type = parse_custom_type("type Color (red, green, blue)").unwrap();
        assert_eq!(enum_type.name(), "Color");
        assert!(enum_type.is_enum()); // Correctly parsed as enum
        assert_eq!(enum_type.as_enum().unwrap().variants().count(), 3);
        let mut variants = enum_type.as_enum().unwrap().variants();
        assert_eq!(*variants.next().unwrap(), "red");
        assert_eq!(*variants.next().unwrap(), "green");
        assert_eq!(*variants.next().unwrap(), "blue");
        assert!(variants.next().is_none());
    }

    #[test]
    fn test_parse_mixed_field_types() {
        // Mixed field types (some with types, some without) should be treated as an error
        let input = "type Mixed (field1, field2: string, field3)";
        let result = parse_custom_type(input);
        assert!(
            result.is_err(),
            "Mixed field types should be a parsing error"
        );
    }

    #[test]
    fn test_parse_empty_custom_type() {
        // Empty custom types should be treated as objects
        let input = "type Empty ()";
        let custom_type = parse_custom_type(input).unwrap();
        assert_eq!(custom_type.name(), "Empty");
        assert!(custom_type.is_object()); // Should be treated as object
        assert!(!custom_type.is_enum()); // Should not be treated as enum
        assert_eq!(custom_type.as_object().unwrap().fields().count(), 0);
    }

    #[test]
    fn test_parse_field() {
        let input = "name: string";
        let field = parse_field(input).unwrap();
        assert_eq!(field.name(), "name");
        assert_eq!(field.ty(), &Type::String);

        let input = "items: []int";
        let field = parse_field(input).unwrap();
        assert_eq!(field.name(), "items");
        assert_eq!(field.ty(), &Type::Array(TypeRef::new(&Type::Int)));
    }

    #[test]
    fn test_parse_field_with_comments() {
        let input = r#"# Field comment
name: string"#;
        let field = parse_field(input).unwrap();
        assert_eq!(field.name(), "name");
        assert_eq!(field.ty(), &Type::String);
        let comments: Vec<_> = field.comments().collect();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text(), "Field comment");

        let input = r#"# First comment
# Second comment
items: []int"#;
        let field = parse_field(input).unwrap();
        assert_eq!(field.name(), "items");
        assert_eq!(field.ty(), &Type::Array(TypeRef::new(&Type::Int)));
        let comments: Vec<_> = field.comments().collect();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text(), "First comment");
        assert_eq!(comments[1].text(), "Second comment");
    }

    #[test]
    fn test_parse_parameter_with_comments() {
        let input = r#"# Parameter comment
name: string"#;
        let param = parse_field(input).unwrap(); // Parameter is alias for Field
        assert_eq!(param.name(), "name");
        assert_eq!(param.ty(), &Type::String);
        let comments: Vec<_> = param.comments().collect();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text(), "Parameter comment");
    }

    #[test]
    fn test_parse_interface() {
        let input = r#"
interface org.example.test

type Person (name: string, age: int)

method GetPerson(id: int) -> (person: Person)

error NotFound(id: int)
        "#;

        let interface = parse_interface(input).unwrap();
        assert_eq!(interface.name(), "org.example.test");
        assert_eq!(interface.members().count(), 3);
    }

    #[test]
    fn parse_error_messages() {
        // Test with invalid syntax
        let invalid_interface = "invalid syntax here";
        let result = parse_interface(invalid_interface);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::Error::IdlParse(msg) => {
                assert!(msg.contains("Parse error"));
            }
            other => panic!("Expected IdlParse error, got: {:?}", other),
        }

        // Test with unexpected remaining input
        let incomplete_interface = "interface com.example.Test\nmethod Test() -> ()\nextra junk";
        let result = parse_interface(incomplete_interface);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::Error::IdlParse(msg) => {
                assert!(msg.contains("Unexpected remaining input") || msg.contains("Parse error"));
            }
            other => panic!("Expected IdlParse error, got: {:?}", other),
        }

        // Test with empty input
        let empty_interface = "";
        let result = parse_interface(empty_interface);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::Error::IdlParse(msg) => {
                assert!(msg.contains("Input is empty"));
            }
            other => panic!("Expected IdlParse error, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_comment() {
        let input = "# This is a comment";
        let mut input_bytes = input.as_bytes();
        let comment = comment_def(&mut input_bytes).unwrap();
        assert_eq!(comment.content(), "This is a comment");
        assert_eq!(comment.text(), "This is a comment");
    }

    #[test]
    fn test_parse_interface_with_comments() {
        let input = r#"
interface org.example.test

# This is a comment
type Person (name: string, age: int)

# Another comment
method GetPerson(id: int) -> (person: Person)

# Final comment
error NotFound(id: int)
        "#;

        let interface = parse_interface(input).unwrap();
        assert_eq!(interface.name(), "org.example.test");

        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 3); // 3 actual members, comments are attached to them

        // Check that members have comments attached
        if let Member::Custom(custom_type) = &members[0] {
            assert_eq!(custom_type.name(), "Person");
            // TODO: Check custom type comments when CustomType supports them
        } else {
            panic!("Expected first member to be a custom type");
        }

        if let Member::Method(method) = &members[1] {
            assert_eq!(method.name(), "GetPerson");
            let comments: Vec<_> = method.comments().collect();
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].text(), "Another comment");
        } else {
            panic!("Expected second member to be a method");
        }

        if let Member::Error(error) = &members[2] {
            assert_eq!(error.name(), "NotFound");
            let comments: Vec<_> = error.comments().collect();
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].text(), "Final comment");
        } else {
            panic!("Expected third member to be an error");
        }
    }

    #[test]
    fn test_ws_with_comments() {
        let input = "  # This is a comment\n  \t# Another comment\n  some_text";
        let mut input_bytes = input.as_bytes();

        // Call ws function
        ws(&mut input_bytes).unwrap();

        // Check that ws consumed whitespace and comments, leaving only "some_text"
        let remaining = core::str::from_utf8(input_bytes).unwrap();
        assert_eq!(remaining, "some_text");

        // Test with just whitespace
        let input2 = "   \t\n   ";
        let mut input_bytes2 = input2.as_bytes();
        ws(&mut input_bytes2).unwrap();
        assert!(input_bytes2.is_empty());

        // Test with just a comment
        let input3 = "# Just a comment\n";
        let mut input_bytes3 = input3.as_bytes();
        ws(&mut input_bytes3).unwrap();
        assert!(input_bytes3.is_empty());
    }

    #[test]
    fn test_parse_simple_enum() {
        let input = "(one, two, three)";
        let mut input_bytes = input.as_bytes();
        match enum_type(&mut input_bytes) {
            Ok(enum_type) => {
                println!("✓ Successfully parsed simple enum: {:?}", enum_type);
            }
            Err(e) => {
                println!("✗ Failed to parse simple enum: {:?}", e);
                panic!("Should be able to parse simple enum: {:?}", e);
            }
        }
    }

    #[test]
    fn test_parse_acquiremetadata_enum_directly() {
        let input = r#"(
	# Do not include metadata in the output
	no,
	# Include metadata in the output
	yes,
	# Include metadata in the output, but gracefully eat up errors
	graceful
)"#;

        let mut input_bytes = input.as_bytes();
        match enum_type(&mut input_bytes) {
            Ok(enum_type) => {
                println!(
                    "✓ Successfully parsed AcquireMetadata enum: {:?}",
                    enum_type
                );
            }
            Err(e) => {
                println!("✗ Failed to parse AcquireMetadata enum: {:?}", e);
                // Print the remaining input to see where it failed
                println!(
                    "Remaining input: {:?}",
                    core::str::from_utf8(input_bytes).unwrap_or("<invalid UTF-8>")
                );
                panic!("Should be able to parse AcquireMetadata enum: {:?}", e);
            }
        }
    }

    #[test]
    fn test_parse_enum_with_comments() {
        let input = r#"type AcquireMetadata(
	# Do not include metadata in the output
	no,
	# Include metadata in the output
	yes,
	# Include metadata in the output, but gracefully eat up errors
	graceful
)"#;

        match parse_custom_type(input) {
            Ok(custom_type) => {
                assert_eq!(custom_type.name(), "AcquireMetadata");
                println!("✓ Successfully parsed enum with comments: {}", custom_type);
            }
            Err(e) => {
                println!("✗ Failed to parse enum with comments: {}", e);
                panic!("Should be able to parse enum with comments: {}", e);
            }
        }
    }

    #[test]
    fn test_multiple_consecutive_comments() {
        let input = r#"
interface org.example.test

# First comment
# Second comment
# Third comment
method SimpleMethod() -> ()

# Fourth comment
# Fifth comment
error SimpleError()
        "#;

        let interface = parse_interface(input).unwrap();
        assert_eq!(interface.name(), "org.example.test");

        let members: Vec<_> = interface.members().collect();
        // Should have: method + error = 2 members (comments attached to them)
        assert_eq!(members.len(), 2);

        // Verify method has multiple comments attached
        if let Member::Method(method) = &members[0] {
            assert_eq!(method.name(), "SimpleMethod");
            let comments: Vec<_> = method.comments().collect();
            assert_eq!(comments.len(), 3);
            assert_eq!(comments[0].text(), "First comment");
            assert_eq!(comments[1].text(), "Second comment");
            assert_eq!(comments[2].text(), "Third comment");
        } else {
            panic!("Expected first member to be a method");
        }

        // Verify error has multiple comments attached
        if let Member::Error(error) = &members[1] {
            assert_eq!(error.name(), "SimpleError");
            let comments: Vec<_> = error.comments().collect();
            assert_eq!(comments.len(), 2);
            assert_eq!(comments[0].text(), "Fourth comment");
            assert_eq!(comments[1].text(), "Fifth comment");
        } else {
            panic!("Expected second member to be an error");
        }
    }

    #[test]
    fn test_comments_attached_to_members() {
        let input = r#"
interface org.example.test

# Documentation for Person type
type Person (name: string, age: int)

# Documentation for GetPerson method
method GetPerson(id: int) -> (person: Person)

# Documentation for NotFound error
error NotFound(id: int)
        "#;

        let interface = parse_interface(input).unwrap();
        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 3);

        // Check that each member has its documentation comment
        if let Member::Custom(custom_type) = &members[0] {
            assert_eq!(custom_type.name(), "Person");
            // Note: CustomType doesn't support comments yet, this will be implemented later
        }

        if let Member::Method(method) = &members[1] {
            assert_eq!(method.name(), "GetPerson");
            let comments: Vec<_> = method.comments().collect();
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].text(), "Documentation for GetPerson method");
        }

        if let Member::Error(error) = &members[2] {
            assert_eq!(error.name(), "NotFound");
            let comments: Vec<_> = error.comments().collect();
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].text(), "Documentation for NotFound error");
        }
    }

    #[test]
    fn test_comprehensive_comment_parsing() {
        let input = r#"
interface org.example.comprehensive

# Type documentation - first line
# Type documentation - second line
type Config (
    # Host configuration
    host: string,
    port: int,
    # Enable/disable flag
    enabled: bool
)

# Method documentation - line 1
# Method documentation - line 2
# Method documentation - line 3
method Configure(config: Config) -> (success: bool)

# Single error comment
error ConfigurationError(message: string, code: int)

# Reset method documentation
method Reset() -> ()

# GetStatus method documentation
method GetStatus() -> (status: string)
        "#;

        let interface = parse_interface(input).unwrap();
        assert_eq!(interface.name(), "org.example.comprehensive");

        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 5); // type + 3 methods + 1 error

        // Check that the type was parsed correctly (comments ignored for now)
        let Member::Custom(custom_type) = &members[0] else {
            panic!("Expected first member to be a custom type");
        };
        assert_eq!(custom_type.name(), "Config");
        let object = custom_type.as_object().unwrap();
        let fields: Vec<_> = object.fields().collect();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name(), "host");
        assert_eq!(fields[1].name(), "port");
        assert_eq!(fields[2].name(), "enabled");
        // Verify field comments are now parsed and attached
        let field0_comments: Vec<_> = fields[0].comments().collect();
        assert_eq!(field0_comments.len(), 1);
        assert_eq!(field0_comments[0].text(), "Host configuration");

        assert_eq!(fields[1].comments().count(), 0); // port has no comment

        let field2_comments: Vec<_> = fields[2].comments().collect();
        assert_eq!(field2_comments.len(), 1);
        assert_eq!(field2_comments[0].text(), "Enable/disable flag");

        // Check that the method has attached comments
        let Member::Method(method) = &members[1] else {
            panic!("Expected second member to be a method");
        };
        assert_eq!(method.name(), "Configure");
        let comments: Vec<_> = method.comments().collect();
        assert_eq!(comments.len(), 3);
        assert_eq!(comments[0].text(), "Method documentation - line 1");
        assert_eq!(comments[1].text(), "Method documentation - line 2");
        assert_eq!(comments[2].text(), "Method documentation - line 3");

        // Verify method parameters have no comments (internal comments ignored)
        let inputs: Vec<_> = method.inputs().collect();
        let outputs: Vec<_> = method.outputs().collect();
        assert_eq!(inputs[0].comments().count(), 0);
        assert_eq!(outputs[0].comments().count(), 0);

        // Check that the error has attached comments
        let Member::Error(error) = &members[2] else {
            panic!("Expected third member to be an error");
        };
        assert_eq!(error.name(), "ConfigurationError");
        let comments: Vec<_> = error.comments().collect();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text(), "Single error comment");

        // Verify error fields have no comments (no field comments in this test)
        let fields: Vec<_> = error.fields().collect();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name(), "message");
        assert_eq!(fields[0].comments().count(), 0);
        assert_eq!(fields[1].name(), "code");
        assert_eq!(fields[1].comments().count(), 0);

        // Check Reset method with single comment
        // Check that the fourth member (Reset method) has attached comments
        let Member::Method(method) = &members[3] else {
            panic!("Expected fourth member to be a method");
        };
        assert_eq!(method.name(), "Reset");
        let comments: Vec<_> = method.comments().collect();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text(), "Reset method documentation");

        // Verify method has no parameters and no comments on parameters
        assert!(method.has_no_inputs());
        assert!(method.has_no_outputs());

        // Check GetStatus method with single comment
        let Member::Method(method) = &members[4] else {
            panic!("Expected fifth member to be a method");
        };
        assert_eq!(method.name(), "GetStatus");
        let comments: Vec<_> = method.comments().collect();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text(), "GetStatus method documentation");

        // Verify method structure
        assert!(method.has_no_inputs());
        assert!(!method.has_no_outputs());
        let outputs: Vec<_> = method.outputs().collect();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].name(), "status");
        assert_eq!(outputs[0].comments().count(), 0);
    }

    #[test]
    fn test_comment_content_edge_cases() {
        let input = r#"
interface org.example.edgecases

# Comment with special characters: !@#$%^&*()
# Comment with unicode: 🚀 UTF-8 тест
# Comment with whitespace:   spaces   and   tabs
method SpecialMethod() -> ()

#No space after hash
error SpecialError()

#    Leading spaces after hash
method AnotherMethod() -> ()
        "#;

        let interface = parse_interface(input).unwrap();
        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 3);

        // Check method with special characters in comments
        if let Member::Method(method) = &members[0] {
            assert_eq!(method.name(), "SpecialMethod");
            let comments: Vec<_> = method.comments().collect();
            assert_eq!(comments.len(), 3);
            assert_eq!(
                comments[0].text(),
                "Comment with special characters: !@#$%^&*()"
            );
            assert_eq!(comments[1].text(), "Comment with unicode: 🚀 UTF-8 тест");
            assert_eq!(
                comments[2].text(),
                "Comment with whitespace:   spaces   and   tabs"
            );
        }

        // Check error with no space after hash
        if let Member::Error(error) = &members[1] {
            assert_eq!(error.name(), "SpecialError");
            let comments: Vec<_> = error.comments().collect();
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].text(), "No space after hash");
        }

        // Check method with leading spaces after hash
        if let Member::Method(method) = &members[2] {
            assert_eq!(method.name(), "AnotherMethod");
            let comments: Vec<_> = method.comments().collect();
            assert_eq!(comments.len(), 1);
            assert_eq!(comments[0].text(), "Leading spaces after hash");
        }
    }

    /// Parse a Varlink type from a string.
    fn parse_type(input: &str) -> Result<Type<'_>, crate::Error> {
        parse_from_str(input, varlink_type)
    }

    /// Parse a method from a string.
    fn parse_method(input: &str) -> Result<Method<'_>, crate::Error> {
        parse_from_str(input, method_def)
    }

    /// Parse an error from a string.
    fn parse_error(input: &str) -> Result<Error<'_>, crate::Error> {
        parse_from_str(input, error_def)
    }

    /// Parse a custom type from a string.
    fn parse_custom_type(input: &str) -> Result<CustomType<'_>, crate::Error> {
        parse_from_str(input, type_def)
    }

    /// Parse a field from a string.
    fn parse_field(input: &str) -> Result<Field<'_>, crate::Error> {
        parse_from_str(input, field)
    }

    #[test]
    fn test_method_with_parameter_comments() {
        let input = r#"
interface org.example.test

method TestMethod(
# The user's name
name: string,
# The user's age in years
age: int,
# Optional email address
email: ?string
) -> (
# Success message
message: string
)
    "#;

        let interface = parse_interface(input).unwrap();
        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 1);

        let Member::Method(method) = &members[0] else {
            panic!("Expected first member to be a method");
        };

        assert_eq!(method.name(), "TestMethod");

        // Check input parameters
        let inputs: Vec<_> = method.inputs().collect();
        assert_eq!(inputs.len(), 3);

        assert_eq!(inputs[0].name(), "name");
        let name_comments: Vec<_> = inputs[0].comments().collect();
        assert_eq!(name_comments.len(), 1);
        assert_eq!(name_comments[0].text(), "The user's name");

        assert_eq!(inputs[1].name(), "age");
        let age_comments: Vec<_> = inputs[1].comments().collect();
        assert_eq!(age_comments.len(), 1);
        assert_eq!(age_comments[0].text(), "The user's age in years");

        assert_eq!(inputs[2].name(), "email");
        let email_comments: Vec<_> = inputs[2].comments().collect();
        assert_eq!(email_comments.len(), 1);
        assert_eq!(email_comments[0].text(), "Optional email address");

        // Check output parameters
        let outputs: Vec<_> = method.outputs().collect();
        assert_eq!(outputs.len(), 1);

        assert_eq!(outputs[0].name(), "message");
        let message_comments: Vec<_> = outputs[0].comments().collect();
        assert_eq!(message_comments.len(), 1);
        assert_eq!(message_comments[0].text(), "Success message");
    }

    #[test]
    fn test_error_with_field_comments() {
        let input = r#"
interface org.example.test

error TestError(
# Error code
code: int,
# Error message
message: string
)
    "#;

        let interface = parse_interface(input).unwrap();
        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 1);

        let Member::Error(error) = &members[0] else {
            panic!("Expected first member to be an error");
        };

        assert_eq!(error.name(), "TestError");

        let fields: Vec<_> = error.fields().collect();
        assert_eq!(fields.len(), 2);

        assert_eq!(fields[0].name(), "code");
        let code_comments: Vec<_> = fields[0].comments().collect();
        assert_eq!(code_comments.len(), 1);
        assert_eq!(code_comments[0].text(), "Error code");

        assert_eq!(fields[1].name(), "message");
        let message_comments: Vec<_> = fields[1].comments().collect();
        assert_eq!(message_comments.len(), 1);
        assert_eq!(message_comments[0].text(), "Error message");
    }

    #[test]
    fn test_struct_type_with_field_comments() {
        let input = r#"
interface org.example.test

type Person(
# Person's full name
name: string,
# Age in years
age: int,
# Contact information
email: ?string
)
    "#;

        let interface = parse_interface(input).unwrap();
        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 1);

        let Member::Custom(custom_type) = &members[0] else {
            panic!("Expected first member to be a custom type");
        };
        let Some(object) = custom_type.as_object() else {
            panic!("Expected custom type to be an object");
        };

        assert_eq!(object.name(), "Person");

        let fields: Vec<_> = object.fields().collect();
        assert_eq!(fields.len(), 3);

        assert_eq!(fields[0].name(), "name");
        let name_comments: Vec<_> = fields[0].comments().collect();
        assert_eq!(name_comments.len(), 1);
        assert_eq!(name_comments[0].text(), "Person's full name");

        assert_eq!(fields[1].name(), "age");
        let age_comments: Vec<_> = fields[1].comments().collect();
        assert_eq!(age_comments.len(), 1);
        assert_eq!(age_comments[0].text(), "Age in years");

        assert_eq!(fields[2].name(), "email");
        let email_comments: Vec<_> = fields[2].comments().collect();
        assert_eq!(email_comments.len(), 1);
        assert_eq!(email_comments[0].text(), "Contact information");
    }

    #[test]
    fn test_interface_with_comments() {
        let input = r#"
# Interface documentation - line 1
# Interface documentation - line 2
interface org.example.test

method SimpleMethod() -> ()
    "#;

        let interface = parse_interface(input).unwrap();
        assert_eq!(interface.name(), "org.example.test");

        // Check interface comments
        let comments: Vec<_> = interface.comments().collect();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text(), "Interface documentation - line 1");
        assert_eq!(comments[1].text(), "Interface documentation - line 2");

        // Verify the interface still has its members
        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 1);
    }

    #[test]
    fn test_custom_type_with_comments() {
        let input = r#"
# Type documentation - line 1
# Type documentation - line 2
type Person(
    name: string,
    age: int
)
    "#;

        let custom_type = parse_custom_type(input).unwrap();
        assert_eq!(custom_type.name(), "Person");

        // Check that this is an object type
        let Some(object) = custom_type.as_object() else {
            panic!("Expected custom type to be an object");
        };

        // Check type comments
        let comments: Vec<_> = object.comments().collect();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text(), "Type documentation - line 1");
        assert_eq!(comments[1].text(), "Type documentation - line 2");

        // Verify the type still has its fields
        let fields: Vec<_> = object.fields().collect();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name(), "name");
        assert_eq!(fields[1].name(), "age");
    }

    #[test]
    fn test_interface_and_type_comments_integration() {
        let input = r#"
# Main interface documentation
# Describes the core API
interface org.example.core

# User data structure
# Contains basic user information
type User(
    # User's display name
    name: string,
    # User's email address
    email: string
)

# Get user information
# Returns user details by ID
method GetUser(id: int) -> (user: User)
    "#;

        let interface = parse_interface(input).unwrap();
        assert_eq!(interface.name(), "org.example.core");

        // Check interface comments
        let interface_comments: Vec<_> = interface.comments().collect();
        assert_eq!(interface_comments.len(), 2);
        assert_eq!(interface_comments[0].text(), "Main interface documentation");
        assert_eq!(interface_comments[1].text(), "Describes the core API");

        // Check members
        let members: Vec<_> = interface.members().collect();
        assert_eq!(members.len(), 2); // 1 type + 1 method

        // Check the custom type member and its comments
        let Member::Custom(custom_type) = &members[0] else {
            panic!("Expected first member to be a custom type");
        };
        let Some(object) = custom_type.as_object() else {
            panic!("Expected custom type to be an object");
        };

        let type_comments: Vec<_> = object.comments().collect();
        assert_eq!(type_comments.len(), 2);
        assert_eq!(type_comments[0].text(), "User data structure");
        assert_eq!(type_comments[1].text(), "Contains basic user information");

        // Check field comments
        let fields: Vec<_> = object.fields().collect();
        assert_eq!(fields.len(), 2);

        let name_comments: Vec<_> = fields[0].comments().collect();
        assert_eq!(name_comments.len(), 1);
        assert_eq!(name_comments[0].text(), "User's display name");

        let email_comments: Vec<_> = fields[1].comments().collect();
        assert_eq!(email_comments.len(), 1);
        assert_eq!(email_comments[0].text(), "User's email address");

        // Check the method member and its comments
        let Member::Method(method) = &members[1] else {
            panic!("Expected second member to be a method");
        };

        let method_comments: Vec<_> = method.comments().collect();
        assert_eq!(method_comments.len(), 2);
        assert_eq!(method_comments[0].text(), "Get user information");
        assert_eq!(method_comments[1].text(), "Returns user details by ID");
    }
}
