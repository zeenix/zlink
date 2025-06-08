//! Method definitions for Varlink IDL.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::{List, Parameter};

/// A method definition in Varlink IDL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method<'a> {
    /// The name of the method.
    pub name: &'a str,
    /// Input parameters for the method.
    pub inputs: List<'a, Parameter<'a>>,
    /// Output parameters for the method.
    pub outputs: List<'a, Parameter<'a>>,
}

impl<'a> Method<'a> {
    /// Creates a new method with the given name, inputs, and outputs.
    pub fn new(
        name: &'a str,
        inputs: List<'a, Parameter<'a>>,
        outputs: List<'a, Parameter<'a>>,
    ) -> Self {
        Self {
            name,
            inputs,
            outputs,
        }
    }

    /// Returns true if the method has no input parameters.
    pub fn has_no_inputs(&self) -> bool {
        self.inputs.is_empty()
    }

    /// Returns true if the method has no output parameters.
    pub fn has_no_outputs(&self) -> bool {
        self.outputs.is_empty()
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
            write!(f, "{}", param)?;
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
                write!(f, "{}", param)?;
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
    use crate::idl::{Field, Type};

    #[test]
    fn test_method_creation() {
        const INPUT: Parameter<'_> = Field {
            name: "id",
            ty: Type::Int,
        };
        const OUTPUT: Parameter<'_> = Field {
            name: "name",
            ty: Type::String,
        };
        const INPUTS: [&Parameter<'_>; 1] = [&INPUT];
        const OUTPUTS: [&Parameter<'_>; 1] = [&OUTPUT];

        let method = Method::new("GetName", List::Borrowed(&INPUTS), List::Borrowed(&OUTPUTS));
        assert_eq!(method.name, "GetName");
        assert_eq!(method.inputs.len(), 1);
        assert_eq!(method.outputs.len(), 1);
        assert!(!method.has_no_inputs());
        assert!(!method.has_no_outputs());
    }

    #[test]
    fn test_method_no_params() {
        let method = Method::new("Ping", List::default(), List::default());
        assert_eq!(method.name, "Ping");
        assert!(method.has_no_inputs());
        assert!(method.has_no_outputs());
    }

    #[test]
    fn test_method_serialization() {
        let inputs = vec![Field::new("x", Type::Float), Field::new("y", Type::Float)];
        let outputs = vec![Field::new("distance", Type::Float)];

        let method = Method::new("CalculateDistance", List::from(inputs), List::from(outputs));
        let json = serde_json::to_string(&method).unwrap();

        assert_eq!(
            json,
            r#""method CalculateDistance(x: float, y: float) -> (distance: float)""#
        );
    }
}
