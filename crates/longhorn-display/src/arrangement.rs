use std::{error::Error, fmt};

use longhorn_core::{DisplayId, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::DisplayFacts;

const PREFIX: &str = "longhorn-arrangement-v1|";
const EMPTY: &str = "longhorn-arrangement-v1|empty";

/// Versioned deterministic description of currently available displays.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArrangementSignature(String);

impl ArrangementSignature {
    /// Returns the stable empty arrangement.
    #[must_use]
    pub fn empty() -> Self {
        Self(EMPTY.to_string())
    }

    /// Validates a serialized arrangement signature.
    pub fn parse(value: impl Into<String>) -> Result<Self, ArrangementSignatureError> {
        let value = value.into();
        validate_signature(&value)?;
        Ok(Self(value))
    }

    /// Returns the versioned serialized signature.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArrangementSignature {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for ArrangementSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ArrangementSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(de::Error::custom)
    }
}

pub(crate) fn build_signature<'a>(
    displays: impl IntoIterator<Item = (&'a DisplayId, &'a DisplayFacts)>,
) -> ArrangementSignature {
    let mut displays = displays.into_iter().collect::<Vec<_>>();
    displays.sort_by_key(|(id, _)| (*id).clone());
    if displays.is_empty() {
        return ArrangementSignature::empty();
    }

    let mut signature = String::from(PREFIX);
    for (index, (id, facts)) in displays.into_iter().enumerate() {
        if index > 0 {
            signature.push(';');
        }
        let id = id.as_str();
        let full = facts.full_bounds();
        let work = facts.work_area();
        signature.push_str(&format!(
            "{}:{}|{}|{}|{}|{}",
            id.len(),
            id,
            encode_rect(full),
            encode_rect(work),
            facts.scale().thousandths(),
            u8::from(facts.is_main())
        ));
    }

    ArrangementSignature(signature)
}

fn encode_rect(rect: ScreenRect) -> String {
    format!(
        "{},{},{},{}",
        rect.origin().x().get(),
        rect.origin().y().get(),
        rect.size().width(),
        rect.size().height()
    )
}

fn validate_signature(value: &str) -> Result<(), ArrangementSignatureError> {
    if value == EMPTY {
        return Ok(());
    }
    let body = value
        .strip_prefix(PREFIX)
        .ok_or(ArrangementSignatureError::UnsupportedVersion)?;
    if body.is_empty() {
        return Err(ArrangementSignatureError::InvalidShape);
    }

    let mut previous_id: Option<DisplayId> = None;
    for entry in body.split(';') {
        let (length, remainder) = entry
            .split_once(':')
            .ok_or(ArrangementSignatureError::InvalidShape)?;
        let length = length
            .parse::<usize>()
            .map_err(|_| ArrangementSignatureError::InvalidShape)?;
        if remainder.len() < length {
            return Err(ArrangementSignatureError::InvalidShape);
        }
        let (id, fields) = remainder.split_at(length);
        let fields = fields
            .strip_prefix('|')
            .ok_or(ArrangementSignatureError::InvalidShape)?;
        let id = DisplayId::new(id).map_err(|_| ArrangementSignatureError::InvalidShape)?;
        if previous_id.as_ref().is_some_and(|previous| previous >= &id) {
            return Err(ArrangementSignatureError::NonCanonicalOrder);
        }
        previous_id = Some(id);

        let fields = fields.split('|').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(ArrangementSignatureError::InvalidShape);
        }
        parse_rect(fields[0])?;
        parse_rect(fields[1])?;
        ScaleFactor::from_thousandths(
            fields[2]
                .parse::<u32>()
                .map_err(|_| ArrangementSignatureError::InvalidShape)?,
        )
        .map_err(|_| ArrangementSignatureError::InvalidShape)?;
        if !matches!(fields[3], "0" | "1") {
            return Err(ArrangementSignatureError::InvalidShape);
        }
    }
    Ok(())
}

fn parse_rect(value: &str) -> Result<ScreenRect, ArrangementSignatureError> {
    let values = value.split(',').collect::<Vec<_>>();
    if values.len() != 4 {
        return Err(ArrangementSignatureError::InvalidShape);
    }
    let x = values[0]
        .parse::<i32>()
        .map_err(|_| ArrangementSignatureError::InvalidShape)?;
    let y = values[1]
        .parse::<i32>()
        .map_err(|_| ArrangementSignatureError::InvalidShape)?;
    let width = values[2]
        .parse::<u32>()
        .map_err(|_| ArrangementSignatureError::InvalidShape)?;
    let height = values[3]
        .parse::<u32>()
        .map_err(|_| ArrangementSignatureError::InvalidShape)?;
    Ok(ScreenRect::new(
        ScreenPoint::new(x, y),
        ScreenSize::new(width, height),
    ))
}

/// Arrangement-signature validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrangementSignatureError {
    /// The version prefix is not supported.
    UnsupportedVersion,
    /// The payload is malformed.
    InvalidShape,
    /// Display entries are not strictly ordered by canonical identity.
    NonCanonicalOrder,
}

impl fmt::Display for ArrangementSignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion => {
                formatter.write_str("arrangement signature version is unsupported")
            }
            Self::InvalidShape => formatter.write_str("arrangement signature shape is invalid"),
            Self::NonCanonicalOrder => {
                formatter.write_str("arrangement signature display order is not canonical")
            }
        }
    }
}

impl Error for ArrangementSignatureError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DisplayBuiltinStatus, DisplayLabel};

    fn facts(x: i32, main: bool) -> DisplayFacts {
        DisplayFacts::new(
            Some(DisplayLabel::new("Desk").unwrap()),
            main,
            DisplayBuiltinStatus::External,
            ScreenRect::new(ScreenPoint::new(x, 0), ScreenSize::new(1920, 1080)),
            ScreenRect::new(ScreenPoint::new(x, 24), ScreenSize::new(1920, 1056)),
            ScaleFactor::from_thousandths(2000).unwrap(),
        )
    }

    #[test]
    fn generated_signatures_are_strictly_parseable() {
        let a = DisplayId::new("display:a").unwrap();
        let b = DisplayId::new("display:b").unwrap();
        let a_facts = facts(0, true);
        let b_facts = facts(1920, false);
        let signature = build_signature([(&b, &b_facts), (&a, &a_facts)]);

        assert_eq!(
            ArrangementSignature::parse(signature.as_str()),
            Ok(signature)
        );
        assert!(ArrangementSignature::parse("longhorn-arrangement-v2|empty").is_err());
    }
}
