use serde::Serializer;

use crate::iff::{LocationCache, LocationCodeHandle};

pub trait SerializeExtras {
    type Ok;
    type Error;
    fn serialize_location_code(
        self,
        handle: &LocationCodeHandle,
        cache: &LocationCache,
    ) -> Result<Self::Ok, Self::Error>;
}

impl<T: Serializer> SerializeExtras for T {
    fn serialize_location_code(
        self,
        handle: &LocationCodeHandle,
        cache: &LocationCache,
    ) -> Result<<T as Serializer>::Ok, <T as Serializer>::Error> {
        self.serialize_str(cache.get_str(handle).unwrap())
    }

    type Ok = <Self as Serializer>::Ok;
    type Error = <Self as Serializer>::Error;
}

// pub trait SuperSerializer: Serializer {
//     type Ok;
//     type Error;
//     type SerializeSeq;
//     type SerializeTuple;
//     type SerializeTupleStruct;
//     type SerializeTupleVariant;
//     type SerializeMap;
//     type SerializeStruct;
//     type SerializeStructVariant;
//     fn serialize_with_state(
//         &mut self,
//         some_value: i32,
//     ) -> Result<<Self as Serializer>::Ok, <Self as Serializer>::Error>;
// }

// struct SuperSerializerImpl<S: Serializer, C> {
//     default_serializer: S,
//     extra_data: C,
// }

// impl<S: Serializer, C> SuperSerializerImpl<S, C> {
//     // Everything below is forwarding to default_serializer
//     // type Ok = <S as Serializer>::Ok;

//     // type Error = <S as Serializer>::Error;

//     // type SerializeSeq = <S as Serializer>::SerializeSeq;

//     // type SerializeTuple = <S as Serializer>::SerializeTuple;

//     // type SerializeTupleStruct = <S as Serializer>::SerializeTupleStruct;

//     // type SerializeTupleVariant = <S as Serializer>::SerializeTupleVariant;

//     // type SerializeMap = <S as Serializer>::SerializeMap;

//     // type SerializeStruct = <S as Serializer>::SerializeStruct;

//     // type SerializeStructVariant = <S as Serializer>::SerializeStructVariant;

//     fn serialize_bool(
//         self,
//         v: bool,
//     ) -> Result<<Self as serde::Serializer>::Ok, <Self as serde::Serializer>::Error> {
//         self.default_serializer.serialize_bool(v)
//     }

//     fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_i8(v)
//     }

//     fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_i16(v)
//     }

//     fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_i32(v)
//     }

//     fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_i64(v)
//     }

//     fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_u8(v)
//     }

//     fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_u16(v)
//     }

//     fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_u32(v)
//     }

//     fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_u64(v)
//     }

//     fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_f32(v)
//     }

//     fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_f64(v)
//     }

//     fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_char(v)
//     }

//     fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_str(v)
//     }

//     fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_bytes(v)
//     }

//     fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_none()
//     }

//     fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_some(value)
//     }

//     fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_unit()
//     }

//     fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer.serialize_unit_struct(name)
//     }

//     fn serialize_unit_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//     ) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer
//             .serialize_unit_variant(name, variant_index, variant)
//     }

//     fn serialize_newtype_struct<T: ?Sized + Serialize>(
//         self,
//         name: &'static str,
//         value: &T,
//     ) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer
//             .serialize_newtype_struct(name, value)
//     }

//     fn serialize_newtype_variant<T: ?Sized + Serialize>(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         value: &T,
//     ) -> Result<Self::Ok, Self::Error> {
//         self.default_serializer
//             .serialize_newtype_variant(name, variant_index, variant, value)
//     }

//     fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
//         self.default_serializer.serialize_seq(len)
//     }

//     fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
//         self.default_serializer.serialize_tuple(len)
//     }

//     fn serialize_tuple_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleStruct, Self::Error> {
//         self.default_serializer.serialize_tuple_struct(name, len)
//     }

//     fn serialize_tuple_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeTupleVariant, Self::Error> {
//         self.default_serializer
//             .serialize_tuple_variant(name, variant_index, variant, len)
//     }

//     fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
//         self.default_serializer.serialize_map(len)
//     }

//     fn serialize_struct(
//         self,
//         name: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStruct, Self::Error> {
//         self.default_serializer.serialize_struct(name, len)
//     }

//     fn serialize_struct_variant(
//         self,
//         name: &'static str,
//         variant_index: u32,
//         variant: &'static str,
//         len: usize,
//     ) -> Result<Self::SerializeStructVariant, Self::Error> {
//         self.default_serializer
//             .serialize_struct_variant(name, variant_index, variant, len)
//     }

//     fn serialize_with_state(
//         &mut self,
//         some_value: i32,
//     ) -> Result<<Self as Serializer>::Ok, <Self as Serializer>::Error> {
//         todo!()
//     }
//     // End of forwarding
// }
