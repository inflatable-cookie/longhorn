//! Projects licence usability into a banner, or into nothing at all.
//!
//! Unlike the first four domains, there is no Svelte counterpart to compare
//! against — `longhorn-poodle-svelte` has no licence projection. So this one
//! carries no port-versus-source finding. It is the first statement of the
//! rule, not the second.

use longhorn_licence::{Timestamp, Usability};
use poodle_specs::{BannerSpec, StatusTone};

/// Renders one licence timestamp as a date a person can read.
///
/// Injected rather than implemented. `Timestamp` is Unix seconds and its
/// `Display` prints the integer, which is correct for a log and useless in a
/// banner. A webview gets `toLocaleString` from the platform for free; Rust
/// has no equivalent without a date library and a locale, and neither belongs
/// in a projection crate. So the caller supplies it, the same way the host
/// adapter supplies a window backend.
pub trait TimestampFormat {
    /// Formats one point in time for display.
    fn format(&self, at: Timestamp) -> String;
}

impl<F> TimestampFormat for F
where
    F: Fn(Timestamp) -> String,
{
    fn format(&self, at: Timestamp) -> String {
        self(at)
    }
}

/// Projects usability into the banner a surface should show, if any.
///
/// Returns `None` for `Active` and for `InGrace`. Grace is deliberate
/// silence: a renewal that has not yet succeeded, still inside its
/// tolerance, is not something the user can act on, and a backend outage
/// must never look to a paying customer like a licensing failure. This
/// mirrors [`Usability::warrants_attention`] rather than deciding again — the
/// rule belongs to the licence domain and this only renders it.
///
/// The remaining states are all `Danger`. None is a warning: in each the
/// software has stopped being usable, and a gentler tone would misreport the
/// state the application is actually in.
#[must_use]
pub fn usability_banner(usability: &Usability, dates: &impl TimestampFormat) -> Option<BannerSpec> {
    if !usability.warrants_attention() {
        return None;
    }

    let (title, message) = match usability {
        // Unreachable: both are excluded above. Kept explicit rather than
        // collapsed into a catch-all so a new usability state fails to
        // compile here instead of silently acquiring someone else's wording.
        Usability::Active | Usability::InGrace { .. } => return None,
        Usability::UseWindowExpired { at } => (
            "Licence expired",
            format!("The use window passed on {}.", dates.format(*at)),
        ),
        Usability::LeaseLapsed { at } => (
            "Licence could not be renewed",
            format!(
                "Renewal did not succeed and grace ran out on {}.",
                dates.format(*at)
            ),
        ),
        Usability::ClockRefused => (
            "System clock refused",
            "The clock moved backwards far enough that licence checks cannot \
             be trusted. Correct the system time and restart."
                .to_owned(),
        ),
    };

    Some(
        BannerSpec::new()
            .with_tone(StatusTone::Danger)
            .with_title(title)
            .with_message(message),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> Timestamp {
        Timestamp::from_unix_seconds(1_700_000_000)
    }

    fn dates() -> impl TimestampFormat {
        |value: Timestamp| format!("<{}>", value.as_unix_seconds())
    }

    #[test]
    fn a_healthy_licence_shows_nothing() {
        assert!(usability_banner(&Usability::Active, &dates()).is_none());
    }

    #[test]
    fn grace_is_deliberate_silence() {
        // A backend outage must never look like a licensing failure to a
        // paying customer. Grace is usable, so it says nothing.
        let grace = Usability::InGrace { until: at() };
        assert!(grace.is_usable());
        assert!(usability_banner(&grace, &dates()).is_none());
    }

    #[test]
    fn every_unusable_state_is_danger_rather_than_warning() {
        for usability in [
            Usability::UseWindowExpired { at: at() },
            Usability::LeaseLapsed { at: at() },
            Usability::ClockRefused,
        ] {
            let banner = usability_banner(&usability, &dates()).expect("banner");
            assert_eq!(banner.tone, StatusTone::Danger, "{usability:?}");
            assert!(banner.title.is_some(), "{usability:?}");
            assert!(banner.message.is_some(), "{usability:?}");
        }
    }

    #[test]
    fn dates_come_from_the_caller_not_from_display() {
        // `Timestamp`'s own `Display` prints Unix seconds. A banner that
        // showed "1700000000" would be a defect, so the projection never
        // reaches for it.
        let banner =
            usability_banner(&Usability::LeaseLapsed { at: at() }, &dates()).expect("banner");
        let message = banner.message.expect("message");

        assert!(message.contains("<1700000000>"), "{message}");
        assert!(!message.contains(" 1700000000"), "{message}");
    }

    #[test]
    fn a_lapsed_lease_and_an_expired_window_do_not_share_wording() {
        // Two different failures with two different remedies. Collapsing them
        // would send a customer to the wrong place.
        let lapsed =
            usability_banner(&Usability::LeaseLapsed { at: at() }, &dates()).expect("banner");
        let expired =
            usability_banner(&Usability::UseWindowExpired { at: at() }, &dates()).expect("banner");

        assert_ne!(lapsed.title, expired.title);
        assert_ne!(lapsed.message, expired.message);
    }
}
