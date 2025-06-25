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

use super::{custom, Error, Field, Interface, List, Member, Method, Parameter, Type, TypeRef};

#[cfg(feature = "std")]
use std::vec::Vec;

/// Parse whitespace.
fn ws<'a>(input: &mut &'a [u8]) -> ModalResult<(), InputError<&'a [u8]>> {
    multispace0.parse_next(input).map(|_| ())
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
    let name = field_name(input)?;
    ws(input)?;
    literal(":").parse_next(input)?;
    ws(input)?;
    let ty = varlink_type(input)?;
    Ok(Field::new_owned(name, ty))
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

/// Parse a parameter definition: name: type.
/// Parse a parameter: name: type.
fn parameter<'a>(input: &mut &'a [u8]) -> ModalResult<Parameter<'a>, InputError<&'a [u8]>> {
    let name = field_name(input)?;
    ws(input)?;
    literal(":").parse_next(input)?;
    ws(input)?;
    let ty = varlink_type(input)?;
    Ok(Parameter::new_owned(name, ty))
}

/// Parse a parameter list: (param1: type1, param2: type2).
fn parameter_list<'a>(
    input: &mut &'a [u8],
) -> ModalResult<Vec<Parameter<'a>>, InputError<&'a [u8]>> {
    literal("(").parse_next(input)?;
    ws(input)?;
    let params: Vec<Parameter<'a>> =
        separated(0.., parameter, (ws, literal(","), ws)).parse_next(input)?;
    ws(input)?;
    literal(")").parse_next(input)?;
    Ok(params)
}

/// Parse a method definition: method Name(params) -> (returns).
fn method_def<'a>(input: &mut &'a [u8]) -> ModalResult<Method<'a>, InputError<&'a [u8]>> {
    literal("method").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = type_name(input)?;
    ws(input)?;
    let input_params = parameter_list(input)?;
    ws(input)?;
    literal("->").parse_next(input)?;
    ws(input)?;
    let output_params = parameter_list(input)?;

    Ok(Method::new_owned(name, input_params, output_params))
}

/// Parse an error definition: error Name (fields).
fn error_def<'a>(input: &mut &'a [u8]) -> ModalResult<Error<'a>, InputError<&'a [u8]>> {
    literal("error").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = type_name(input)?;
    ws(input)?;
    let params = parameter_list(input)?;

    Ok(Error::new_owned(name, params))
}

/// Parse a type definition: type Name <definition>.
fn type_def<'a>(input: &mut &'a [u8]) -> ModalResult<custom::Type<'a>, InputError<&'a [u8]>> {
    literal("type").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = type_name(input)?;
    ws(input)?;
    literal("(").parse_next(input)?;
    ws(input)?;

    // Parse as struct - type definitions must have typed fields
    let fields: Vec<Field<'a>> = separated(0.., field, (ws, literal(","), ws)).parse_next(input)?;
    ws(input)?;
    literal(")").parse_next(input)?;
    Ok(custom::Type::from(custom::Object::new_owned(name, fields)))
}

/// Parse a member definition (type, method, or error).
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
    literal("interface").parse_next(input)?;
    take_while(1.., |c: u8| c.is_ascii_whitespace()).parse_next(input)?;
    let name = interface_name(input)?;
    ws(input)?;

    // Parse members separated by whitespace/newlines
    let mut members = Vec::new();

    while !input.is_empty() {
        ws(input)?;

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

    Ok(Interface::new_owned(name, members))
}

/// Parse a Varlink type from a string.
pub(super) fn parse_type(input: &str) -> Result<Type<'_>, crate::Error> {
    parse_from_str(input, varlink_type)
}

/// Parse an interface from a string.
pub(super) fn parse_interface(input: &str) -> Result<Interface<'_>, crate::Error> {
    parse_from_str(input, interface_def)
}

/// Parse a member from a string.
pub(super) fn parse_member(input: &str) -> Result<Member<'_>, crate::Error> {
    parse_from_str(input, member_def)
}

/// Parse a method from a string.
pub(super) fn parse_method(input: &str) -> Result<Method<'_>, crate::Error> {
    parse_from_str(input, method_def)
}

/// Parse an error from a string.
pub(super) fn parse_error(input: &str) -> Result<Error<'_>, crate::Error> {
    parse_from_str(input, error_def)
}

/// Parse a custom type from a string.
pub(super) fn parse_custom_type(input: &str) -> Result<custom::Type<'_>, crate::Error> {
    parse_from_str(input, type_def)
}

/// Parse a field from a string.
pub(super) fn parse_field(input: &str) -> Result<Field<'_>, crate::Error> {
    parse_from_str(input, field)
}

/// Helper function to parse from string using byte-based parsers.
fn parse_from_str<'a, T>(
    input: &'a str,
    parser: impl Fn(&mut &'a [u8]) -> ModalResult<T, InputError<&'a [u8]>>,
) -> Result<T, crate::Error> {
    let input_bytes = input.trim().as_bytes();
    if input_bytes.is_empty() {
        return Err(crate::Error::BufferOverflow);
    }

    let mut input_mut = input_bytes;
    match parser(&mut input_mut) {
        Ok(result) => {
            let _ = ws(&mut input_mut);
            if input_mut.is_empty() {
                Ok(result)
            } else {
                Err(crate::Error::BufferOverflow)
            }
        }
        Err(_) => Err(crate::Error::BufferOverflow),
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
            &Parameter::new("info", &Type::String)
        );
        assert!(outputs.next().is_none());

        let input = "method Add(a: int, b: int) -> (sum: int)";
        let method = parse_method(input).unwrap();
        assert_eq!(method.name(), "Add");
        let mut inputs = method.inputs();
        assert_eq!(inputs.next().unwrap(), &Parameter::new("a", &Type::Int));
        assert_eq!(inputs.next().unwrap(), &Parameter::new("b", &Type::Int));
        assert!(inputs.next().is_none());
        let mut outputs = method.outputs();
        assert_eq!(outputs.next().unwrap(), &Parameter::new("sum", &Type::Int));
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
            &Field::new("resource", &Type::String)
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
        assert_eq!(fields.next().unwrap(), &Field::new("name", &Type::String));
        assert_eq!(fields.next().unwrap(), &Field::new("age", &Type::Int));
        assert!(fields.next().is_none());

        let input = "type Config (host: string, port: int, enabled: bool)";
        let custom_type = parse_custom_type(input).unwrap();
        assert_eq!(custom_type.name(), "Config");
        assert_eq!(custom_type.as_object().unwrap().fields().count(), 3);
        let mut fields = custom_type.as_object().unwrap().fields();
        assert_eq!(fields.next().unwrap(), &Field::new("host", &Type::String));
        assert_eq!(fields.next().unwrap(), &Field::new("port", &Type::Int));
        assert_eq!(fields.next().unwrap(), &Field::new("enabled", &Type::Bool));
        assert!(fields.next().is_none());

        // Invalid: enum-style definitions are not allowed for type definitions
        assert!(parse_custom_type("type Color (red, green, blue)").is_err());
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
    fn test_deserialize_functionality() {
        // Test that serde_json deserialization works correctly
        let method_json = r#""method GetInfo() -> (info: string)""#;
        let method: Method<'_> = serde_json::from_str(method_json).unwrap();
        assert_eq!(method.name(), "GetInfo");
        assert_eq!(method.inputs().count(), 0);
        assert_eq!(method.outputs().count(), 1);

        let error_json = r#""error NotFound(id: int)""#;
        let error: Error<'_> = serde_json::from_str(error_json).unwrap();
        assert_eq!(error.name(), "NotFound");
        let mut fields = error.fields();
        assert_eq!(fields.next().unwrap(), &Field::new("id", &Type::Int));
        assert!(fields.next().is_none());

        let custom_type_json = r#""type Person (name: string, age: int)""#;
        let custom_type: custom::Type<'_> = serde_json::from_str(custom_type_json).unwrap();
        assert_eq!(custom_type.name(), "Person");
        let mut fields = custom_type.as_object().unwrap().fields();
        assert_eq!(fields.next().unwrap(), &Field::new("name", &Type::String));
        assert_eq!(fields.next().unwrap(), &Field::new("age", &Type::Int));
        assert!(fields.next().is_none());

        let field_json = r#""name: string""#;
        let field: Field<'_> = serde_json::from_str(field_json).unwrap();
        assert_eq!(field.name(), "name");
        assert_eq!(field.ty(), &Type::String);
    }
}
