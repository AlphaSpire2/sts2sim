//! The digest behind [`ObservationKey`](crate::ObservationKey): a value's
//! serde data model written as compact, self-delimiting bytes and hashed
//! with XXH3-128.
//!
//! The key sits on a search's innermost loop, so it is built for speed
//! rather than for reading: no text formatting, no escaping, and a
//! non-cryptographic hash over one reused buffer. What it may not give up
//! is separation. Every value is tagged with its kind, every string and
//! byte run is length-prefixed, and every compound is closed by an end tag,
//! so two different data-model streams never write the same bytes. Field
//! and variant names are written as text, so the bytes depend only on the
//! value's types and contents and the key is the same in every process and
//! on every platform.
//!
//! A struct writes exactly as a map with string keys does. That keeps a
//! `#[serde(flatten)]` field, which serde serializes through a map, from
//! keying differently than the same fields written inline.

use std::cell::RefCell;
use std::fmt::{self, Display};

use serde::ser::{self, Serialize};

/// Kind tags. Each value written starts with exactly one.
mod tag {
    pub const BOOL: u8 = 1;
    pub const I8: u8 = 2;
    pub const I16: u8 = 3;
    pub const I32: u8 = 4;
    pub const I64: u8 = 5;
    pub const I128: u8 = 6;
    pub const U8: u8 = 7;
    pub const U16: u8 = 8;
    pub const U32: u8 = 9;
    pub const U64: u8 = 10;
    pub const U128: u8 = 11;
    pub const F32: u8 = 12;
    pub const F64: u8 = 13;
    pub const CHAR: u8 = 14;
    pub const STR: u8 = 15;
    pub const BYTES: u8 = 16;
    pub const NONE: u8 = 17;
    pub const SOME: u8 = 18;
    pub const UNIT: u8 = 19;
    pub const UNIT_STRUCT: u8 = 20;
    pub const UNIT_VARIANT: u8 = 21;
    pub const NEWTYPE_STRUCT: u8 = 22;
    pub const NEWTYPE_VARIANT: u8 = 23;
    pub const SEQ: u8 = 24;
    pub const TUPLE: u8 = 25;
    pub const TUPLE_STRUCT: u8 = 26;
    pub const TUPLE_VARIANT: u8 = 27;
    /// Maps and structs alike, so a flattened struct keys as its fields
    /// written inline.
    pub const MAP: u8 = 28;
    pub const STRUCT_VARIANT: u8 = 29;
    pub const END: u8 = 0xFF;
}

/// The 32 bytes of a key: two XXH3-128 digests of `value`'s encoding under
/// different seeds.
pub(crate) fn digest<T: Serialize + ?Sized>(value: &T) -> Result<[u8; 32], Error> {
    thread_local! {
        static BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }
    BUFFER.with(|buffer| {
        // Taken rather than borrowed, so a key computed while another is
        // being written on the same thread cannot find the buffer in use.
        let mut encoder = Encoder(std::mem::take(&mut *buffer.borrow_mut()));
        encoder.0.clear();
        let written = value.serialize(&mut encoder);
        let bytes = encoder.0;
        let digest = written.map(|()| {
            let mut key = [0; 32];
            key[..16].copy_from_slice(&twox_hash::XxHash3_128::oneshot(&bytes).to_le_bytes());
            key[16..].copy_from_slice(
                &twox_hash::XxHash3_128::oneshot_with_seed(1, &bytes).to_le_bytes(),
            );
            key
        });
        *buffer.borrow_mut() = bytes;
        digest
    })
}

/// A `Serialize` implementation refused to write itself. The encoder has no
/// failure of its own.
#[derive(Debug)]
pub(crate) struct Error(String);

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "observation key encoding: {}", self.0)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

struct Encoder(Vec<u8>);

impl Encoder {
    fn tagged(&mut self, tag: u8, bytes: &[u8]) {
        self.0.push(tag);
        self.0.extend_from_slice(bytes);
    }

    fn text(&mut self, tag: u8, bytes: &[u8]) {
        self.tagged(tag, &(bytes.len() as u64).to_le_bytes());
        self.0.extend_from_slice(bytes);
    }

    fn variant(&mut self, tag: u8, index: u32, name: &str) {
        self.tagged(tag, &index.to_le_bytes());
        self.text(tag::STR, name.as_bytes());
    }
}

macro_rules! number {
    ($method:ident, $type:ty, $tag:expr) => {
        fn $method(self, value: $type) -> Result<(), Error> {
            self.tagged($tag, &value.to_le_bytes());
            Ok(())
        }
    };
}

impl ser::Serializer for &mut Encoder {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, value: bool) -> Result<(), Error> {
        self.tagged(tag::BOOL, &[u8::from(value)]);
        Ok(())
    }

    number!(serialize_i8, i8, tag::I8);
    number!(serialize_i16, i16, tag::I16);
    number!(serialize_i32, i32, tag::I32);
    number!(serialize_i64, i64, tag::I64);
    number!(serialize_i128, i128, tag::I128);
    number!(serialize_u8, u8, tag::U8);
    number!(serialize_u16, u16, tag::U16);
    number!(serialize_u32, u32, tag::U32);
    number!(serialize_u64, u64, tag::U64);
    number!(serialize_u128, u128, tag::U128);
    number!(serialize_f32, f32, tag::F32);
    number!(serialize_f64, f64, tag::F64);

    fn serialize_char(self, value: char) -> Result<(), Error> {
        self.tagged(tag::CHAR, &u32::from(value).to_le_bytes());
        Ok(())
    }

    fn serialize_str(self, value: &str) -> Result<(), Error> {
        self.text(tag::STR, value.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<(), Error> {
        self.text(tag::BYTES, value);
        Ok(())
    }

    fn serialize_none(self) -> Result<(), Error> {
        self.0.push(tag::NONE);
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Error> {
        self.0.push(tag::SOME);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Error> {
        self.0.push(tag::UNIT);
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<(), Error> {
        self.text(tag::UNIT_STRUCT, name.as_bytes());
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.variant(tag::UNIT_VARIANT, index, variant);
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.0.push(tag::NEWTYPE_STRUCT);
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.variant(tag::NEWTYPE_VARIANT, index, variant);
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self, Error> {
        self.0.push(tag::SEQ);
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self, Error> {
        self.0.push(tag::TUPLE);
        Ok(self)
    }

    fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self, Error> {
        self.0.push(tag::TUPLE_STRUCT);
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self, Error> {
        self.variant(tag::TUPLE_VARIANT, index, variant);
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self, Error> {
        self.0.push(tag::MAP);
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self, Error> {
        self.0.push(tag::MAP);
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self, Error> {
        self.variant(tag::STRUCT_VARIANT, index, variant);
        Ok(self)
    }
}

macro_rules! elements {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for &mut Encoder {
            type Ok = ();
            type Error = Error;

            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
                value.serialize(&mut **self)
            }

            fn end(self) -> Result<(), Error> {
                self.0.push(tag::END);
                Ok(())
            }
        }
    };
}

elements!(SerializeSeq, serialize_element);
elements!(SerializeTuple, serialize_element);
elements!(SerializeTupleStruct, serialize_field);
elements!(SerializeTupleVariant, serialize_field);

impl ser::SerializeMap for &mut Encoder {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        self.0.push(tag::END);
        Ok(())
    }
}

macro_rules! fields {
    ($trait:ident) => {
        impl ser::$trait for &mut Encoder {
            type Ok = ();
            type Error = Error;

            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), Error> {
                self.text(tag::STR, key.as_bytes());
                value.serialize(&mut **self)
            }

            fn end(self) -> Result<(), Error> {
                self.0.push(tag::END);
                Ok(())
            }
        }
    };
}

fields!(SerializeStruct);
fields!(SerializeStructVariant);

#[cfg(test)]
mod tests {
    use super::digest;
    use serde::Serialize;
    use std::collections::BTreeMap;

    #[derive(Serialize)]
    struct Inline {
        a: u32,
        b: &'static str,
    }

    #[derive(Serialize)]
    struct Outer {
        #[serde(flatten)]
        inner: Inline,
    }

    #[test]
    fn a_flattened_struct_keys_as_its_fields_written_inline() {
        let inline = Inline { a: 7, b: "x" };
        let flattened = Outer {
            inner: Inline { a: 7, b: "x" },
        };
        assert_eq!(digest(&inline).unwrap(), digest(&flattened).unwrap());
    }

    #[test]
    fn neighbouring_strings_do_not_run_together() {
        assert_ne!(digest(&("ab", "c")).unwrap(), digest(&("a", "bc")).unwrap());
        assert_ne!(
            digest(&vec![vec![1u8], vec![]]).unwrap(),
            digest(&vec![vec![], vec![1u8]]).unwrap()
        );
    }

    #[test]
    fn absent_and_empty_and_unit_differ() {
        let none: Option<()> = None;
        let empty: Vec<u8> = Vec::new();
        let distinct = [
            digest(&none).unwrap(),
            digest(&Some(())).unwrap(),
            digest(&()).unwrap(),
            digest(&empty).unwrap(),
            digest(&BTreeMap::<u8, u8>::new()).unwrap(),
        ];
        for (index, key) in distinct.iter().enumerate() {
            assert!(!distinct[index + 1..].contains(key));
        }
    }

    #[test]
    fn numbers_of_different_widths_differ() {
        assert_ne!(digest(&1u8).unwrap(), digest(&1u16).unwrap());
        assert_ne!(digest(&0.0f64).unwrap(), digest(&-0.0f64).unwrap());
    }
}
