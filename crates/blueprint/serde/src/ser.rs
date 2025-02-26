use crate::error::{Result, UnsupportedType};
use crate::Field;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use serde::ser;
use serde::Serialize;
use tangle_subxt::FieldExt;
use tangle_subxt::subxt_core::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::{BoundedString, FieldType};

/// A serializer for [`Field`]
///
/// See [`crate::into_field`].
pub struct Serializer;

impl<'a> serde::Serializer for &'a mut Serializer {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;
    type SerializeSeq = SerializeSeq<'a>;
    type SerializeTuple = Self::SerializeSeq;
    type SerializeTupleStruct = SerializeTupleStruct<'a>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = SerializeStruct<'a>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

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
        Ok(Field::List(FieldType::Uint8, BoundedVec(
            v.iter().map(|b| Field::Uint8(*b)).collect(),
        )))
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        Ok(Field::Optional(FieldType::Void, Box::new(None)))
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

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let vec;
        match len {
            Some(len) => vec = Vec::with_capacity(len),
            None => vec = Vec::new(),
        }

        Ok(SerializeSeq { ser: self, determined_type: None, vec })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
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
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Self::Error::UnsupportedType(UnsupportedType::Map))
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
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
    ) -> Result<Self::SerializeStructVariant> {
        Err(Self::Error::UnsupportedType(UnsupportedType::NonUnitEnum))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub struct SerializeSeq<'a> {
    ser: &'a mut Serializer,
    vec: Vec<Field<AccountId32>>,
    determined_type: Option<FieldType>,
}

impl SerializeSeq<'_> {
    fn is_homogeneous(&self) -> bool {
        if self.vec.is_empty() {
            return true;
        }

        macro_rules! homogeneous_check {
            ($($field:pat),+ $(,)?) => {
                paste::paste! {
                    match &self.vec[0] {
                        $($field => { self.vec.iter().all(|f| matches!(f, $field)) },)+
                        Field::Struct(name, fields) => {
                            self.vec.iter().all(|f| {
                                match f {
                                    Field::Struct(n, f) => {
                                        n == name && fields == f
                                    },
                                    _ => false,
                                }
                            })
                        }
                    }
                }
            }
        }

        homogeneous_check!(
            Field::Optional(..),
            Field::Bool(_),
            Field::Uint8(_),
            Field::Int8(_),
            Field::Uint16(_),
            Field::Int16(_),
            Field::Uint32(_),
            Field::Int32(_),
            Field::Uint64(_),
            Field::Int64(_),
            Field::String(_),
            Field::Array(..),
            Field::List(..),
            Field::AccountId(_),
        )
    }
}

impl ser::SerializeSeq for SerializeSeq<'_> {
    type Ok = Field<AccountId32>;
    type Error = crate::error::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let value = value.serialize(&mut *self.ser)?;
        if self.determined_type.is_none() {
            self.determined_type = Some(value.field_type());
        }

        self.vec.push(value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(Field::List(self.determined_type.unwrap_or(FieldType::Void), BoundedVec(self.vec)))
    }
}

impl ser::SerializeTuple for SerializeSeq<'_> {
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
        if self.is_homogeneous() {
            return Ok(Field::Array(self.determined_type.unwrap_or(FieldType::Void), BoundedVec(self.vec)));
        }

        Err(crate::error::Error::HeterogeneousTuple)
    }
}

pub struct SerializeTupleStruct<'a> {
    ser: &'a mut Serializer,
    name: &'a str,
    fields: Vec<(BoundedString, Field<AccountId32>)>,
}

impl ser::SerializeTupleStruct for SerializeTupleStruct<'_> {
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

impl ser::SerializeStruct for SerializeStruct<'_> {
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

pub fn new_bounded_string<S>(s: S) -> BoundedString
where
    S: Into<String>,
{
    let s = s.into();
    BoundedString(BoundedVec(s.into_bytes()))
}
