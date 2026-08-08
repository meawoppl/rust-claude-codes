//! serde adapters for the quirks of the protobuf-JSON encoding.
//!
//! The harness serialises its protobuf messages with the canonical JSON
//! mapping, which departs from what `#[derive(Deserialize)]` would assume in
//! three places:
//!
//! - **64-bit integers are strings.** `{"seqNum": "17"}`, not `{"seqNum": 17}`.
//!   Decoders are expected to accept both, so [`opt_int`] and [`vec_int`] take
//!   either and always emit the string form.
//! - **`bytes` is base64.** [`opt_bytes`] and [`vec_bytes`] decode standard and
//!   URL-safe alphabets, with or without padding, and emit standard padded.
//! - **Enums are value names**, though numbers are also legal input. Generated
//!   enums decode through [`EnumRepr`] and keep unrecognised values rather than
//!   failing, so a harness newer than this crate stays readable.

use std::fmt::Display;
use std::str::FromStr;

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine as _;
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serializer};

/// How a protobuf enum arrived on the wire: by name, or by number.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum EnumRepr {
    /// The canonical form — the proto value name, e.g. `"STATE_DONE"`.
    Name(String),
    /// The numeric form, which the JSON mapping also permits.
    Number(i32),
}

fn decode_base64<E: de::Error>(s: &str) -> Result<Vec<u8>, E> {
    for engine in [&STANDARD, &STANDARD_NO_PAD, &URL_SAFE, &URL_SAFE_NO_PAD] {
        if let Ok(bytes) = engine.decode(s) {
            return Ok(bytes);
        }
    }
    Err(E::invalid_value(
        Unexpected::Str(s),
        &"base64-encoded bytes",
    ))
}

struct IntVisitor<T>(std::marker::PhantomData<T>);

impl<T> Visitor<'_> for IntVisitor<T>
where
    T: FromStr + TryFrom<i64> + TryFrom<u64>,
    <T as FromStr>::Err: Display,
{
    type Value = T;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an integer, as a JSON number or a decimal string")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<T, E> {
        v.parse().map_err(|e: <T as FromStr>::Err| E::custom(e))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<T, E> {
        T::try_from(v).map_err(|_| E::custom(format!("integer {v} out of range")))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<T, E> {
        T::try_from(v).map_err(|_| E::custom(format!("integer {v} out of range")))
    }
}

fn deserialize_int<'de, T, D>(d: D) -> Result<T, D::Error>
where
    T: FromStr + TryFrom<i64> + TryFrom<u64>,
    <T as FromStr>::Err: Display,
    D: Deserializer<'de>,
{
    d.deserialize_any(IntVisitor(std::marker::PhantomData))
}

/// A 64-bit integer that may be absent, encoded as a JSON string.
pub mod opt_int {
    use super::*;

    /// Emits the canonical string form, or `null` when absent.
    pub fn serialize<T: Display, S: Serializer>(v: &Option<T>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(v) => s.serialize_str(&v.to_string()),
            None => s.serialize_none(),
        }
    }

    /// Accepts a string, a number, or `null`.
    pub fn deserialize<'de, T, D>(d: D) -> Result<Option<T>, D::Error>
    where
        T: FromStr + TryFrom<i64> + TryFrom<u64>,
        <T as FromStr>::Err: Display,
        D: Deserializer<'de>,
    {
        struct OptVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> Visitor<'de> for OptVisitor<T>
        where
            T: FromStr + TryFrom<i64> + TryFrom<u64>,
            <T as FromStr>::Err: Display,
        {
            type Value = Option<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an optional integer")
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                super::deserialize_int(d).map(Some)
            }
        }

        d.deserialize_option(OptVisitor(std::marker::PhantomData))
    }
}

/// A repeated 64-bit integer field, encoded as JSON strings.
pub mod vec_int {
    use super::*;

    /// Emits each element in the canonical string form.
    pub fn serialize<T: Display, S: Serializer>(v: &[T], s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for item in v {
            seq.serialize_element(&item.to_string())?;
        }
        seq.end()
    }

    /// Accepts a sequence whose elements are strings or numbers.
    pub fn deserialize<'de, T, D>(d: D) -> Result<Vec<T>, D::Error>
    where
        T: FromStr + TryFrom<i64> + TryFrom<u64>,
        <T as FromStr>::Err: Display,
        D: Deserializer<'de>,
    {
        struct SeqVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> Visitor<'de> for SeqVisitor<T>
        where
            T: FromStr + TryFrom<i64> + TryFrom<u64>,
            <T as FromStr>::Err: Display,
        {
            type Value = Vec<T>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a sequence of integers")
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Vec::new())
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::with_capacity(a.size_hint().unwrap_or(0));
                while let Some(v) = a.next_element_seed(IntSeed(std::marker::PhantomData))? {
                    out.push(v);
                }
                Ok(out)
            }
        }

        struct IntSeed<T>(std::marker::PhantomData<T>);

        impl<'de, T> de::DeserializeSeed<'de> for IntSeed<T>
        where
            T: FromStr + TryFrom<i64> + TryFrom<u64>,
            <T as FromStr>::Err: Display,
        {
            type Value = T;

            fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<T, D::Error> {
                super::deserialize_int(d)
            }
        }

        d.deserialize_any(SeqVisitor(std::marker::PhantomData))
    }
}

/// A `bytes` field that may be absent, encoded as base64.
pub mod opt_bytes {
    use super::*;

    /// Emits standard padded base64, or `null` when absent.
    pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(b) => s.serialize_str(&STANDARD.encode(b)),
            None => s.serialize_none(),
        }
    }

    /// Accepts standard or URL-safe base64, padded or not.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        let raw = Option::<String>::deserialize(d)?;
        raw.map(|s| decode_base64(&s)).transpose()
    }
}

/// A repeated `bytes` field, encoded as base64 strings.
pub mod vec_bytes {
    use super::*;

    /// Emits each element as standard padded base64.
    pub fn serialize<S: Serializer>(v: &[Vec<u8>], s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for item in v {
            seq.serialize_element(&STANDARD.encode(item))?;
        }
        seq.end()
    }

    /// Accepts a sequence of base64 strings.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec<u8>>, D::Error> {
        let raw = Option::<Vec<String>>::deserialize(d)?.unwrap_or_default();
        raw.iter().map(|s| decode_base64(s)).collect()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        #[serde(
            default,
            with = "super::opt_int",
            skip_serializing_if = "Option::is_none"
        )]
        seq: Option<i64>,
        #[serde(
            default,
            with = "super::opt_bytes",
            skip_serializing_if = "Option::is_none"
        )]
        data: Option<Vec<u8>>,
        #[serde(
            default,
            with = "super::vec_int",
            skip_serializing_if = "Vec::is_empty"
        )]
        counts: Vec<u64>,
    }

    #[test]
    fn accepts_string_and_numeric_integers() {
        let from_string: Sample = serde_json::from_str(r#"{"seq":"17"}"#).unwrap();
        let from_number: Sample = serde_json::from_str(r#"{"seq":17}"#).unwrap();
        assert_eq!(from_string.seq, Some(17));
        assert_eq!(from_number.seq, from_string.seq);
    }

    #[test]
    fn emits_the_canonical_string_form() {
        let s = Sample {
            seq: Some(-3),
            data: None,
            counts: vec![1, 2],
        };
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"seq":"-3","counts":["1","2"]}"#
        );
    }

    #[test]
    fn absent_fields_stay_absent() {
        let s: Sample = serde_json::from_str("{}").unwrap();
        assert_eq!(
            s,
            Sample {
                seq: None,
                data: None,
                counts: vec![]
            }
        );
    }

    #[test]
    fn base64_round_trips_across_alphabets() {
        let padded: Sample = serde_json::from_str(r#"{"data":"//79"}"#).unwrap();
        let url_safe: Sample = serde_json::from_str(r#"{"data":"__79"}"#).unwrap();
        assert_eq!(padded.data, Some(vec![0xff, 0xfe, 0xfd]));
        assert_eq!(url_safe.data, padded.data);
        assert_eq!(
            serde_json::to_string(&padded).unwrap(),
            r#"{"data":"//79"}"#
        );
    }

    #[test]
    fn null_decodes_as_absent() {
        let s: Sample = serde_json::from_str(r#"{"seq":null,"data":null}"#).unwrap();
        assert_eq!(s.seq, None);
        assert_eq!(s.data, None);
    }
}
