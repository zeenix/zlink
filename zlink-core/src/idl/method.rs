//! Method definitions for Varlink IDL.

use core::fmt;

use serde::Serialize;

#[cfg(feature = "idl-parse")]
use serde::Deserialize;

use super::{Comment, List, Parameter};

/// A method definition in Varlink IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method<'a> {
    /// The name of the method.
    name: &'a str,
    /// Input parameters for the method.
    inputs: List<'a, Parameter<'a>>,
    /// Output parameters for the method.
    outputs: List<'a, Parameter<'a>>,
    /// Comments associated with this method.
    comments: List<'a, Comment<'a>>,
}

impl<'a> Method<'a> {
    /// Creates a new method with the given name, borrowed parameters, and comments.
    pub const fn new(
        name: &'a str,
        inputs: &'a [&'a Parameter<'a>],
        outputs: &'a [&'a Parameter<'a>],
        comments: &'a [&'a Comment<'a>],
    ) -> Self {
        Self {
            name,
            inputs: List::Borrowed(inputs),
            outputs: List::Borrowed(outputs),
            comments: List::Borrowed(comments),
        }
    }

    /// Creates a new method with the given name, owned parameters, and comments.
    #[cfg(feature = "std")]
    pub fn new_owned(
        name: &'a str,
        inputs: Vec<Parameter<'a>>,
        outputs: Vec<Parameter<'a>>,
        comments: Vec<Comment<'a>>,
    ) -> Self {
        Self {
            name,
            inputs: List::from(inputs),
            outputs: List::from(outputs),
            comments: List::from(comments),
        }
    }

    /// Returns the name of the method.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns an iterator over the input parameters.
    pub fn inputs(&self) -> impl Iterator<Item = &Parameter<'a>> {
        self.inputs.iter()
    }

    /// Returns an iterator over the output parameters.
    pub fn outputs(&self) -> impl Iterator<Item = &Parameter<'a>> {
        self.outputs.iter()
    }

    /// Returns true if the method has no input parameters.
    pub fn has_no_inputs(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Returns true if the method has no output parameters.
    pub fn has_no_outputs(&self) -> bool {
        self.outputs.is_empty()
    }

    /// Returns the comments associated with this method.
    pub fn comments(&self) -> impl Iterator<Item = &Comment<'a>> {
        self.comments.iter()
    }
}

impl<'a> fmt::Display for Method<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "method {}(", self.name)?;
        let mut first = true;
        for param in self.inputs.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{param}")?;
        }
        write!(f, ")")?;

        if !self.has_no_outputs() {
            write!(f, " -> (")?;
            let mut first = true;
            for param in self.outputs.iter() {
                if !first {
                    write!(f, ", ")?;
                }
                first = false;
                write!(f, "{param}")?;
            }
            write!(f, ")")?;
        }

        Ok(())
    }
}

impl<'a> Serialize for Method<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "idl-parse")]
impl<'de, 'a> Deserialize<'de> for Method<'a>
where
    'de: 'a,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        super::parse::parse_method(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{Parameter, Type};

    #[test]
    fn method_creation() {
        let input = Parameter::new("id", &Type::Int, &[]);
        let output = Parameter::new("name", &Type::String, &[]);
        let inputs = [&input];
        let outputs = [&output];

        let method = Method::new("GetName", &inputs, &outputs, &[]);
        assert_eq!(method.name(), "GetName");
        assert_eq!(method.inputs().count(), 1);
        assert_eq!(method.outputs().count(), 1);
        assert!(!method.has_no_inputs());
        assert!(!method.has_no_outputs());

        // Check the parameters individually - order and values.
        let inputs_vec: mayheap::Vec<_, 8> = method.inputs().collect();
        assert_eq!(inputs_vec[0].name(), "id");
        assert_eq!(inputs_vec[0].ty(), &Type::Int);

        let outputs_vec: mayheap::Vec<_, 8> = method.outputs().collect();
        assert_eq!(outputs_vec[0].name(), "name");
        assert_eq!(outputs_vec[0].ty(), &Type::String);
    }

    #[test]
    fn method_no_params() {
        let method = Method::new("Ping", &[], &[], &[]);
        assert_eq!(method.name(), "Ping");
        assert!(method.has_no_inputs());
        assert!(method.has_no_outputs());
    }

    #[test]
    fn method_serialization() {
        let input_x = Parameter::new("x", &Type::Float, &[]);
        let input_y = Parameter::new("y", &Type::Float, &[]);
        let output = Parameter::new("distance", &Type::Float, &[]);
        let inputs = [&input_x, &input_y];
        let outputs = [&output];

        let method = Method::new("CalculateDistance", &inputs, &outputs, &[]);

        // Check the parameters individually - order and values.
        let inputs_vec: mayheap::Vec<_, 8> = method.inputs().collect();
        assert_eq!(inputs_vec[0].name(), "x");
        assert_eq!(inputs_vec[0].ty(), &Type::Float);
        assert_eq!(inputs_vec[1].name(), "y");
        assert_eq!(inputs_vec[1].ty(), &Type::Float);

        let outputs_vec: mayheap::Vec<_, 8> = method.outputs().collect();
        assert_eq!(outputs_vec[0].name(), "distance");
        assert_eq!(outputs_vec[0].ty(), &Type::Float);

        #[cfg(feature = "std")]
        let json = serde_json::to_string(&method).unwrap();
        #[cfg(feature = "embedded")]
        let json = {
            let mut buffer = [0u8; 128];
            let len = serde_json_core::to_slice(&method, &mut buffer).unwrap();
            let vec = mayheap::Vec::<_, 128>::from_slice(&buffer[..len]).unwrap();
            mayheap::String::<128>::from_utf8(vec).unwrap()
        };
        assert_eq!(
            json,
            r#""method CalculateDistance(x: float, y: float) -> (distance: float)""#
        );
    }
}
