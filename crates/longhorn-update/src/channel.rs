use core::fmt;

use semver::Version;
use serde::{Deserialize, Serialize};

/// A release line.
///
/// All channels ship under one bundle identity, so an install moves between
/// them without reinstalling and without a second copy of its data.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum Channel {
    /// The default line.
    #[default]
    Production,
    /// Pre-release candidates.
    Beta,
    /// Continuous builds.
    Nightly,
}

impl Channel {
    /// Every channel, in increasing order of change.
    pub const ALL: [Self; 3] = [Self::Production, Self::Beta, Self::Nightly];

    /// Returns the stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }

    /// Returns whether staged rollout applies to this channel.
    ///
    /// Only production stages. Being an early recipient is the entire point
    /// of the other two, so withholding from them would be incoherent.
    #[must_use]
    pub const fn stages_rollout(self) -> bool {
        matches!(self, Self::Production)
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// What this build is, for update checks and for diagnostics attribution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildIdentity {
    /// The channel this build was produced for.
    pub channel: Channel,
    /// The version this build reports.
    pub version: Version,
}

impl BuildIdentity {
    /// Records one build identity.
    #[must_use]
    pub const fn new(channel: Channel, version: Version) -> Self {
        Self { channel, version }
    }

    /// Stamps the running build into the diagnostics seam.
    ///
    /// A bug report that does not say which channel produced it costs more to
    /// triage than the report is worth, and nightly is the line most likely to
    /// generate one.
    pub fn stamp_diagnostics(&self) {
        longhorn_core::report_best_effort_failure(
            "update.build-identity",
            format_args!("{} {}", self.channel, self.version),
        );
    }
}

impl fmt::Display for BuildIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.channel, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_production_stages_rollout() {
        assert!(Channel::Production.stages_rollout());
        assert!(!Channel::Beta.stages_rollout());
        assert!(!Channel::Nightly.stages_rollout());
    }

    #[test]
    fn channels_round_trip_as_kebab_case() {
        for channel in Channel::ALL {
            let json = serde_json::to_string(&channel).unwrap();
            assert_eq!(json, format!("\"{}\"", channel.as_str()));
            assert_eq!(serde_json::from_str::<Channel>(&json).unwrap(), channel);
        }
    }
}
