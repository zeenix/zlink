use core::{fmt, marker::PhantomData};

use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor, value::MapDeserializer},
};

/// The parameters of a method call, which the call may leave out.
///
/// The `parameters` of a method call are optional and callers differ in what they send when they
/// have nothing to pass: some leave them out, others send an empty object. This deserializes a `T`
/// from the parameters and, if the call has none (or `null`), from an empty object. `T` is then
/// built from its optional fields only, or fails to be built if it has required ones.
///
/// It is meant to be the content of a newtype variant of an adjacently tagged enum
/// (`#[serde(tag = "method", content = "parameters")]`), since `serde` doesn't accept an empty
/// object as the content of a unit variant (see <https://github.com/serde-rs/serde/issues/2045>)
/// and requires the content of a struct variant to be there, even if all its fields are optional.
/// This is a workaround for that, to be removed once `serde` handles these cases itself.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct OptionalParams<T>(pub T);

impl<'de, T> Deserialize<'de> for OptionalParams<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(ParamsVisitor(PhantomData))
    }
}

/// Visits the parameters like an `Option` would: `None` stands for the parameters being left out.
///
/// An adjacently tagged enum asks for an `Option` when the content of a newtype variant is missing,
/// and the content can also be `null`.
struct ParamsVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for ParamsVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = OptionalParams<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the parameters of a method call")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let empty = MapDeserializer::<_, E>::new(core::iter::empty::<(&str, ())>());

        T::deserialize(empty).map(OptionalParams)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(OptionalParams)
    }
}
