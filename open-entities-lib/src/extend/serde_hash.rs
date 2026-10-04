//! The state hash through serde: a `Serializer` that feeds [`StateHasher`] and refuses floats,
//! and a probing `Deserializer` that builds sample values of a type so the refusal happens at
//! registration instead of in the middle of a match.
//!
//! A boundary module: it names `f32` and `f64`, but only to reject them.

use std::cell::{Cell, RefCell};
use std::fmt::Display;

use serde::de::{self, DeserializeOwned, DeserializeSeed, IntoDeserializer, Visitor};
use serde::ser::{self, Serialize};

use crate::state_hash::StateHasher;

/// How deep the probe follows options, lists and maps; enough for any sane component, and a stop
/// for recursive types.
const MAX_DEPTH: u32 = 8;

/// Probing rounds before giving up on enums with very many variants.
const MAX_ROUNDS: u32 = 256;

/// Why a value or a type cannot go into the hash.
#[derive(Debug)]
pub(super) struct HashError(String);

impl Display for HashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HashError {}

impl ser::Error for HashError {
    fn custom<T: Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

impl de::Error for HashError {
    fn custom<T: Display>(msg: T) -> Self {
        Self(msg.to_string())
    }
}

/// Writes `value` into `hasher`.
pub(super) fn write<T: Serialize + ?Sized>(
    value: &T,
    hasher: &mut StateHasher,
) -> Result<(), HashError> {
    value.serialize(&mut HashSerializer {
        hasher,
        path: Vec::new(),
    })
}

/// Checks that `T` hashes: builds sample values through `T`'s `Deserialize` impl — every enum
/// variant at least once, options as `Some`, one element per list and map — refusing floats on the
/// way, then serializes each sample into a scratch hasher, refusing floats there too.
pub(super) fn check<T: Serialize + DeserializeOwned>() -> Result<(), String> {
    let state = ProbeState::default();
    loop {
        state.more_variants.set(false);
        state.path.borrow_mut().clear();
        let sample = T::deserialize(Probe {
            state: &state,
            depth: 0,
        })
        .map_err(|err| err.0)?;
        write(&sample, &mut StateHasher::new()).map_err(|err| err.0)?;
        if !state.more_variants.get() || state.round.get() >= MAX_ROUNDS {
            return Ok(());
        }
        state.round.set(state.round.get() + 1);
    }
}

fn float_error(kind: &str, path: &[String]) -> HashError {
    let at = if path.is_empty() {
        String::from("the component itself")
    } else {
        format!("`{}`", path.join("."))
    };
    HashError(format!(
        "{at} is {kind}; simulation state is integers (store milli-units in an i32)"
    ))
}

// --- Serializer ---------------------------------------------------------------------------------

struct HashSerializer<'a> {
    hasher: &'a mut StateHasher,
    /// Field names down to the value being written, for the error message.
    path: Vec<String>,
}

impl HashSerializer<'_> {
    fn len(&mut self, len: Option<usize>) -> Result<(), HashError> {
        let len = len.ok_or_else(|| {
            HashError(format!(
                "a list or map at `{}` does not know its length up front",
                self.path.join(".")
            ))
        })?;
        self.hasher
            .write_u64(u64::try_from(len).expect("a length fits u64"));
        Ok(())
    }

    fn variant(&mut self, index: u32) {
        self.hasher.write_u32(index);
    }
}

impl ser::Serializer for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<(), HashError> {
        self.hasher.write_u8(u8::from(v));
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<(), HashError> {
        self.hasher.write_bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<(), HashError> {
        self.hasher.write_bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<(), HashError> {
        self.hasher.write_i32(v);
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<(), HashError> {
        self.hasher.write_bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i128(self, v: i128) -> Result<(), HashError> {
        self.hasher.write_bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<(), HashError> {
        self.hasher.write_u8(v);
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<(), HashError> {
        self.hasher.write_bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<(), HashError> {
        self.hasher.write_u32(v);
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<(), HashError> {
        self.hasher.write_u64(v);
        Ok(())
    }
    fn serialize_u128(self, v: u128) -> Result<(), HashError> {
        self.hasher.write_bytes(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_f32(self, _: f32) -> Result<(), HashError> {
        Err(float_error("f32", &self.path))
    }
    fn serialize_f64(self, _: f64) -> Result<(), HashError> {
        Err(float_error("f64", &self.path))
    }
    fn serialize_char(self, v: char) -> Result<(), HashError> {
        self.hasher.write_u32(u32::from(v));
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<(), HashError> {
        self.serialize_bytes(v.as_bytes())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<(), HashError> {
        self.len(Some(v.len()))?;
        self.hasher.write_bytes(v);
        Ok(())
    }
    fn serialize_none(self) -> Result<(), HashError> {
        self.hasher.write_u8(0);
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), HashError> {
        self.hasher.write_u8(1);
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), HashError> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), HashError> {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
    ) -> Result<(), HashError> {
        self.variant(index);
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        self.variant(index);
        self.path.push(variant.to_owned());
        value.serialize(&mut *self)?;
        self.path.pop();
        Ok(())
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self, HashError> {
        self.len(len)?;
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self, HashError> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, HashError> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, HashError> {
        self.variant(index);
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self, HashError> {
        self.len(len)?;
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, HashError> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, HashError> {
        self.variant(index);
        Ok(self)
    }
}

impl ser::SerializeSeq for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), HashError> {
        Ok(())
    }
}

impl ser::SerializeTuple for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), HashError> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), HashError> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), HashError> {
        Ok(())
    }
}

impl ser::SerializeMap for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), HashError> {
        key.serialize(&mut **self)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashError> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<(), HashError> {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        self.path.push(key.to_owned());
        value.serialize(&mut **self)?;
        self.path.pop();
        Ok(())
    }
    fn end(self) -> Result<(), HashError> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut HashSerializer<'_> {
    type Ok = ();
    type Error = HashError;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), HashError> {
        self.path.push(key.to_owned());
        value.serialize(&mut **self)?;
        self.path.pop();
        Ok(())
    }
    fn end(self) -> Result<(), HashError> {
        Ok(())
    }
}

// --- Probe --------------------------------------------------------------------------------------

#[derive(Default)]
struct ProbeState {
    /// Round `n` picks variant `min(n, last)` of every enum.
    round: Cell<u32>,
    /// Set when some enum has a variant this round did not reach.
    more_variants: Cell<bool>,
    /// Field and variant names down to the value being built, for the error message.
    path: RefCell<Vec<String>>,
}

/// A `Deserializer` that makes up a value of whatever type asks: zeros, empty strings, `Some`,
/// one-element lists and maps, and an error for floats.
#[derive(Clone, Copy)]
struct Probe<'s> {
    state: &'s ProbeState,
    depth: u32,
}

impl Probe<'_> {
    const fn deeper(self) -> Self {
        Self {
            state: self.state,
            depth: self.depth + 1,
        }
    }

    const fn fill(self) -> bool {
        self.depth < MAX_DEPTH
    }

    fn float(self, kind: &str) -> HashError {
        float_error(kind, &self.state.path.borrow())
    }

    fn named<T>(
        self,
        name: &str,
        f: impl FnOnce() -> Result<T, HashError>,
    ) -> Result<T, HashError> {
        self.state.path.borrow_mut().push(name.to_owned());
        let result = f();
        if result.is_ok() {
            self.state.path.borrow_mut().pop();
        }
        result
    }
}

impl<'de> de::Deserializer<'de> for Probe<'_> {
    type Error = HashError;

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, HashError> {
        let path = self.state.path.borrow();
        Err(HashError(format!(
            "the type at `{}` decides its shape from the data (deserialize_any: untagged enums, \
             flattened fields, dynamic values), so it cannot be checked for floats; implement \
             StateHash by hand",
            path.join(".")
        )))
    }
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_bool(false)
    }
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_i8(0)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_i16(0)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_i32(0)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_i64(0)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_i128(0)
    }
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_u8(0)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_u16(0)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_u32(0)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_u64(0)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_u128(0)
    }
    fn deserialize_f32<V: Visitor<'de>>(self, _: V) -> Result<V::Value, HashError> {
        Err(self.float("f32"))
    }
    fn deserialize_f64<V: Visitor<'de>>(self, _: V) -> Result<V::Value, HashError> {
        Err(self.float("f64"))
    }
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_char('\0')
    }
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_str("")
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_str("")
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_bytes(&[])
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_bytes(&[])
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        if self.fill() {
            visitor.visit_some(self.deeper())
        } else {
            visitor.visit_none()
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, HashError> {
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, HashError> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        let len = usize::from(self.fill());
        visitor.visit_seq(ProbeSeq::new(self.deeper(), len, None))
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, HashError> {
        visitor.visit_seq(ProbeSeq::new(self, len, None))
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, HashError> {
        visitor.visit_seq(ProbeSeq::new(self, len, None))
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        let len = usize::from(self.fill());
        visitor.visit_map(ProbeSeq::new(self.deeper(), len, None))
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, HashError> {
        visitor.visit_seq(ProbeSeq::new(self, fields.len(), Some(fields)))
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, HashError> {
        let last = variants.len().saturating_sub(1);
        let round = usize::try_from(self.state.round.get()).unwrap_or(usize::MAX);
        if round < last {
            self.state.more_variants.set(true);
        }
        let index = round.min(last);
        let name = variants.get(index).copied().unwrap_or_default();
        visitor.visit_enum(ProbeEnum {
            probe: self,
            index: u32::try_from(index).expect("variant index fits u32"),
            name,
        })
    }
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_u64(0)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, HashError> {
        visitor.visit_unit()
    }
}

/// `len` probe values in a row, named after `fields` when the row is a struct's.
struct ProbeSeq<'s> {
    probe: Probe<'s>,
    len: usize,
    next: usize,
    fields: Option<&'static [&'static str]>,
}

impl<'s> ProbeSeq<'s> {
    const fn new(probe: Probe<'s>, len: usize, fields: Option<&'static [&'static str]>) -> Self {
        Self {
            probe,
            len,
            next: 0,
            fields,
        }
    }

    fn element<'de, T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, HashError> {
        let name = self
            .fields
            .and_then(|fields| fields.get(self.next))
            .copied();
        self.next += 1;
        let probe = self.probe;
        match name {
            Some(name) => probe.named(name, || seed.deserialize(probe)),
            None => seed.deserialize(probe),
        }
    }
}

impl<'de> de::SeqAccess<'de> for ProbeSeq<'_> {
    type Error = HashError;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, HashError> {
        if self.next == self.len {
            return Ok(None);
        }
        self.element(seed).map(Some)
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.len - self.next)
    }
}

impl<'de> de::MapAccess<'de> for ProbeSeq<'_> {
    type Error = HashError;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, HashError> {
        if self.next == self.len {
            return Ok(None);
        }
        seed.deserialize(self.probe).map(Some)
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, HashError> {
        self.element(seed)
    }
}

struct ProbeEnum<'s> {
    probe: Probe<'s>,
    index: u32,
    name: &'static str,
}

impl<'de> de::EnumAccess<'de> for ProbeEnum<'_> {
    type Error = HashError;
    type Variant = Self;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), HashError> {
        let variant = seed.deserialize(self.index.into_deserializer())?;
        Ok((variant, self))
    }
}

impl<'de> de::VariantAccess<'de> for ProbeEnum<'_> {
    type Error = HashError;
    fn unit_variant(self) -> Result<(), HashError> {
        Ok(())
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, HashError> {
        let probe = self.probe.deeper();
        probe.named(self.name, || seed.deserialize(probe))
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, HashError> {
        let probe = self.probe.deeper();
        probe.named(self.name, || {
            visitor.visit_seq(ProbeSeq::new(probe, len, None))
        })
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, HashError> {
        let probe = self.probe.deeper();
        probe.named(self.name, || {
            visitor.visit_seq(ProbeSeq::new(probe, fields.len(), Some(fields)))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct Plain {
        a: u8,
        b: i64,
        name: String,
        tags: Vec<u16>,
        by_id: BTreeMap<u32, bool>,
        maybe: Option<(i32, char)>,
    }

    #[derive(Serialize, Deserialize)]
    enum Mode {
        Idle,
        Patrol(Vec<i32>),
        Guard { radius: i32, speed: f32 },
    }

    #[derive(Serialize, Deserialize)]
    struct Tree {
        children: Vec<Tree>,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    enum Loose {
        Number(u32),
        Text(String),
    }

    fn hash<T: Serialize>(value: &T) -> u64 {
        let mut hasher = StateHasher::new();
        write(value, &mut hasher).expect("hashes");
        hasher.finish()
    }

    #[test]
    fn integer_types_pass() {
        assert_eq!(check::<Plain>(), Ok(()));
        assert_eq!(check::<u32>(), Ok(()));
    }

    #[test]
    fn recursive_types_terminate() {
        assert_eq!(check::<Tree>(), Ok(()));
    }

    #[test]
    fn every_enum_variant_is_probed() {
        let err = check::<Mode>().unwrap_err();
        assert!(err.contains("Guard.speed") && err.contains("f32"), "{err}");
    }

    #[test]
    fn shapes_decided_by_the_data_are_refused() {
        let err = check::<Loose>().unwrap_err();
        assert!(err.contains("deserialize_any"), "{err}");
    }

    #[test]
    fn the_serializer_refuses_floats_too() {
        let err = write(&1.5_f64, &mut StateHasher::new()).unwrap_err();
        assert!(err.0.contains("f64"), "{err}");
    }

    #[test]
    fn integers_hash_like_the_hand_written_impls() {
        let mut by_hand = StateHasher::new();
        by_hand.write_i32(-7);
        by_hand.write_u32(9);
        assert_eq!(hash(&(-7_i32, 9_u32)), by_hand.finish());
    }

    #[test]
    fn variants_and_lengths_change_the_hash() {
        assert_ne!(hash(&Mode::Idle), hash(&Mode::Patrol(Vec::new())));
        assert_ne!(hash(&vec![1_i32]), hash(&vec![1_i32, 0]));
        assert_ne!(hash(&Some(0_u8)), hash(&None::<u8>));
    }
}
