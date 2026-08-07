use core::fmt;
use std::{collections::BTreeMap, error::Error};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{Channel, Rollout};

/// A platform an artifact is built for, as `<os>-<arch>`.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct TargetTriple(String);

impl From<TargetTriple> for String {
    fn from(value: TargetTriple) -> Self {
        value.0
    }
}

impl TargetTriple {
    /// Validates and records a target.
    pub fn new(value: impl Into<String>) -> Result<Self, TargetTripleError> {
        let value = value.into();
        if value.is_empty() {
            return Err(TargetTripleError::Empty);
        }
        Ok(Self(value))
    }

    /// Returns the target.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TargetTriple {
    type Error = TargetTripleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Target validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetTripleError {
    /// The target was empty.
    Empty,
}

impl fmt::Display for TargetTripleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("target must not be empty")
    }
}

impl Error for TargetTripleError {}

/// One downloadable build.
///
/// The signature travels with the artifact and is verified by the installer,
/// never by this crate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    /// Where the artifact can be fetched.
    pub url: String,
    /// Detached signature, passed through to the installer unread.
    pub signature: String,
}

impl Artifact {
    /// Records an artifact.
    #[must_use]
    pub fn new(url: impl Into<String>, signature: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            signature: signature.into(),
        }
    }
}

/// What a channel currently publishes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelManifest {
    /// The channel this manifest describes.
    pub channel: Channel,
    /// The version it publishes.
    pub version: Version,
    /// Release notes, when published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Installs below this version update regardless of rollout.
    ///
    /// The security-fix lever. Rollout limits the blast radius of a bad
    /// release; this is what overrides it when waiting is the larger risk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_version: Option<Version>,
    /// Staged rollout. Absent means the release is offered in full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollout: Option<Rollout>,
    /// Artifacts by target.
    #[serde(default)]
    pub artifacts: BTreeMap<TargetTriple, Artifact>,
}

impl ChannelManifest {
    /// Records a manifest with no rollout, floor, or notes.
    #[must_use]
    pub fn new(channel: Channel, version: Version) -> Self {
        Self {
            channel,
            version,
            notes: None,
            minimum_version: None,
            rollout: None,
            artifacts: BTreeMap::new(),
        }
    }

    /// Sets release notes.
    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Sets the mandatory-update floor.
    #[must_use]
    pub fn with_minimum_version(mut self, minimum: Version) -> Self {
        self.minimum_version = Some(minimum);
        self
    }

    /// Sets the staged rollout.
    #[must_use]
    pub fn with_rollout(mut self, rollout: Rollout) -> Self {
        self.rollout = Some(rollout);
        self
    }

    /// Adds one target's artifact.
    #[must_use]
    pub fn with_artifact(mut self, target: TargetTriple, artifact: Artifact) -> Self {
        self.artifacts.insert(target, artifact);
        self
    }

    /// Returns the artifact for one target.
    #[must_use]
    pub fn artifact(&self, target: &TargetTriple) -> Option<&Artifact> {
        self.artifacts.get(target)
    }

    /// Returns whether an installed version is below the mandatory floor.
    #[must_use]
    pub fn is_below_minimum(&self, installed: &Version) -> bool {
        self.minimum_version
            .as_ref()
            .is_some_and(|minimum| installed < minimum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(value: &str) -> Version {
        Version::parse(value).unwrap()
    }

    #[test]
    fn a_manifest_without_a_floor_is_never_below_it() {
        let manifest = ChannelManifest::new(Channel::Production, version("1.3.0"));

        assert!(!manifest.is_below_minimum(&version("0.1.0")));
    }

    #[test]
    fn the_floor_is_exclusive_of_itself() {
        let manifest = ChannelManifest::new(Channel::Production, version("1.3.0"))
            .with_minimum_version(version("1.2.4"));

        assert!(manifest.is_below_minimum(&version("1.2.3")));
        assert!(!manifest.is_below_minimum(&version("1.2.4")));
        assert!(!manifest.is_below_minimum(&version("1.2.5")));
    }

    #[test]
    fn optional_fields_stay_out_of_the_wire_form() {
        let manifest = ChannelManifest::new(Channel::Production, version("1.3.0"));
        let json = serde_json::to_string(&manifest).unwrap();

        assert!(!json.contains("notes"));
        assert!(!json.contains("minimumVersion"));
        assert!(!json.contains("rollout"));
        assert_eq!(
            serde_json::from_str::<ChannelManifest>(&json).unwrap(),
            manifest
        );
    }

    #[test]
    fn empty_targets_are_refused() {
        assert_eq!(TargetTriple::new(""), Err(TargetTripleError::Empty));
    }
}
