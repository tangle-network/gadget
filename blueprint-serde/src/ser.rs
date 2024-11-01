use crate::error::{Result, UnsupportedType};
use crate::Field;
use serde::ser;
use serde::Serialize;
use std::fmt::Display;
use tangle_subxt::subxt_core::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString;

pub struct Serializer;

impl<'a> serde::Serializer for &'a mut Serializer {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;
    type SerializeSeq = SerializeSeq<'a>;
    type SerializeTuple = Self::SerializeSeq;
    type SerializeTupleStruct = SerializeTupleStruct<'a>;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = SerializeStruct<'a>;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        Ok(Field::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        Ok(Field::Int8(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        Ok(Field::Int16(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        Ok(Field::Int32(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        Ok(Field::Int64(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        Ok(Field::Uint8(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        Ok(Field::Uint16(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        Ok(Field::Uint32(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        Ok(Field::Uint64(v))
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
        Err(Self::Error::UnsupportedType(UnsupportedType::f32))
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        Err(Self::Error::UnsupportedType(UnsupportedType::f64))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        Ok(Field::String(new_bounded_string(v)))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        Ok(Field::String(new_bounded_string(v)))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        Ok(Field::Bytes(BoundedVec(v.into())))
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        Ok(Field::None)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        Ok(Field::String(new_bounded_string(variant)))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        let vec;
        match len {
            Some(len) => vec = Vec::with_capacity(len),
            None => vec = Vec::new(),
        }

        Ok(SerializeSeq { ser: self, vec })
    }

    fn serialize_tuple(self, len: usize) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Self::Error> {
        let ser = SerializeTupleStruct {
            ser: self,
            name,
            fields: Vec::with_capacity(len),
        };
        Ok(ser)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }

    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        Err(Self::Error::UnsupportedType(UnsupportedType::Map))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        let ser = SerializeStruct {
            ser: self,
            name,
            fields: Vec::with_capacity(len),
        };
        Ok(ser)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }

    fn collect_str<T>(self, _value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Display,
    {
        todo!()
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct SerializeSeq<'a> {
    ser: &'a mut Serializer,
    vec: Vec<Field<AccountId32>>,
}

impl<'a> ser::SerializeSeq for SerializeSeq<'a> {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(&mut *self.ser)?;
        self.vec.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Field::List(BoundedVec(self.vec)))
    }
}

impl<'a> ser::SerializeTuple for SerializeSeq<'a> {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        <SerializeSeq<'_> as ser::SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        <SerializeSeq<'_> as ser::SerializeSeq>::end(self)
    }
}

pub struct SerializeTupleStruct<'a> {
    ser: &'a mut Serializer,
    name: &'a str,
    fields: Vec<(BoundedString, Field<AccountId32>)>,
}

impl<'a> ser::SerializeTupleStruct for SerializeTupleStruct<'a> {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let field_value = value.serialize(&mut *self.ser)?;
        let field_name = format!("field_{}", self.fields.len());
        self.fields
            .push((new_bounded_string(field_name), field_value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Field::Struct(
            new_bounded_string(self.name),
            Box::new(BoundedVec(self.fields)),
        ))
    }
}

pub struct SerializeStruct<'a> {
    ser: &'a mut Serializer,
    name: &'a str,
    fields: Vec<(BoundedString, Field<AccountId32>)>,
}

impl<'a> ser::SerializeStruct for SerializeStruct<'a> {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let field_value = value.serialize(&mut *self.ser)?;
        self.fields.push((new_bounded_string(key), field_value));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Field::Struct(
            new_bounded_string(self.name),
            Box::new(BoundedVec(self.fields)),
        ))
    }
}

// === UNSUPPORTED TYPES ===

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }

    fn end(self) -> Result<Self::Ok> {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Self::Error::UnsupportedType(UnsupportedType::Map))
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Self::Error::UnsupportedType(UnsupportedType::Map))
    }

    fn end(self) -> Result<Self::Ok> {
        Err(Self::Error::UnsupportedType(UnsupportedType::Map))
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }

    fn end(self) -> Result<Self::Ok> {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }
}

pub(crate) fn new_bounded_string<S>(s: S) -> BoundedString
where
    S: Into<String>,
{
    let s = s.into();
    BoundedString(BoundedVec(s.into_bytes()))
}
