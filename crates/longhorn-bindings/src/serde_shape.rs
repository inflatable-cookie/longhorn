//! Reads external enum names directly from Serde's derived deserializer.

use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, de, forward_to_deserialize_any};
use serde_json::Value;

use crate::generation::Artifact;

#[derive(Debug)]
enum Shape {
    Variants(&'static [&'static str]),
    Unsupported(String),
}

impl fmt::Display for Shape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Variants(_) => formatter.write_str("captured Serde enum variants"),
            Self::Unsupported(message) => formatter.write_str(message),
        }
    }
}

impl Error for Shape {}

impl de::Error for Shape {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Unsupported(message.to_string())
    }
}

struct EnumNames;

impl<'de> Deserializer<'de> for EnumNames {
    type Error = Shape;

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Shape::Variants(variants))
    }

    fn deserialize_any<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(Shape::Unsupported(
            "Serde did not describe an externally tagged enum".into(),
        ))
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf option
        unit unit_struct newtype_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

pub(crate) fn enum_names<T: for<'de> Deserialize<'de>>()
-> Result<&'static [&'static str], Box<dyn Error>> {
    match T::deserialize(EnumNames) {
        Err(Shape::Variants(names)) => Ok(names),
        Err(error) => Err(error.into()),
        Ok(_) => Err("Serde accepted an empty shape probe".into()),
    }
}

pub(crate) fn enum_names_json<T: for<'de> Deserialize<'de>>() -> Result<String, Box<dyn Error>> {
    Ok(serde_json::to_string(enum_names::<T>()?)?)
}

pub(crate) fn error_declarations<T: Serialize, C: for<'de> Deserialize<'de>>(
    error: T,
    name: &str,
    code_name: &str,
) -> Result<[String; 2], Box<dyn Error>> {
    let names = enum_names::<C>()?;
    let variants = names
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    let Value::Object(fields) = serde_json::to_value(error)? else {
        return Err(format!("{name} must serialize as an object").into());
    };
    if !matches!(fields.get("code"), Some(Value::String(code)) if names.contains(&code.as_str())) {
        return Err(format!("{name} must serialize a recognized {code_name}").into());
    }
    let fields = fields
        .iter()
        .map(|(field, value)| {
            let kind = match (field.as_str(), value) {
                ("code", _) => code_name,
                (_, Value::String(_)) => "string",
                (_, Value::Bool(_)) => "boolean",
                _ => return Err(format!("unsupported field in {name}: {field}").into()),
            };
            Ok(format!("{}: {kind},", serde_json::to_string(field)?))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    Ok([
        format!("type {code_name} = {};", variants.join(" | ")),
        format!("type {name} = {{ {} }};", fields.join(" ")),
    ])
}

pub(crate) fn error_fixture<T: Serialize + for<'de> Deserialize<'de> + Eq>(
    relative_path: &'static str,
    samples: &[T],
) -> Result<Artifact, Box<dyn Error>> {
    let json = serde_json::to_string_pretty(samples)?;
    if serde_json::from_str::<Vec<T>>(&json)? != samples {
        return Err(format!("host errors in {relative_path} must round-trip through Serde").into());
    }
    Ok(Artifact {
        relative_path,
        contents: format!("{json}\n"),
    })
}
