use serde::{
    ser::{Error, Impossible, SerializeMap, SerializeStruct},
    Serialize, Serializer,
};

use super::Call;

impl<M> Serialize for Call<M>
where
    M: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(4))?;

        let flat_ser = FlatSerializer(&mut map);
        self.method.serialize(flat_ser)?;

        if self.oneway {
            map.serialize_entry("oneway", &true)?;
        }
        if self.more {
            map.serialize_entry("more", &true)?;
        }
        if self.upgrade {
            map.serialize_entry("upgrade", &true)?;
        }

        map.end()
    }
}

struct FlatSerializer<'a, M: SerializeMap>(&'a mut M);

impl<'a, M> Serializer for FlatSerializer<'a, M>
where
    M: SerializeMap,
{
    type Ok = ();
    type Error = M::Error;

    type SerializeMap = Self;
    type SerializeStruct = Self;
    // we only support `map` and `struct`
    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    // … you’d do the same for serialize_i32, serialize_str, etc.

    // entry-point for T’s Map or Struct
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    // Dummy impl for all other serializer methods.
    fn serialize_bool(self, _: bool) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
    fn collect_str<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + core::fmt::Display,
    {
        Err(<M as SerializeMap>::Error::custom(ERR_MESSAGE))
    }
}

impl<M> SerializeMap for FlatSerializer<'_, M>
where
    M: SerializeMap,
{
    type Ok = ();
    type Error = M::Error;

    // now forward keys & values into the real map
    fn serialize_key<K>(&mut self, key: &K) -> Result<(), Self::Error>
    where
        K: ?Sized + Serialize,
    {
        self.0.serialize_key(key)
    }

    fn serialize_value<V>(&mut self, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        self.0.serialize_value(value)
    }

    // end of inner map
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<M> SerializeStruct for FlatSerializer<'_, M>
where
    M: SerializeMap,
{
    type Ok = ();
    type Error = M::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.0.serialize_key(key)?;
        self.0.serialize_value(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

const ERR_MESSAGE: &str =
    "Must serialize as a map or struct with 2 fields/entries: `method` and `parameters`";
