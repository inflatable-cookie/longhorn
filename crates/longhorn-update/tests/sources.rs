//! Source adapter acceptance evidence.

use longhorn_update::{
    Artifact, BuildIdentity, Channel, ChannelManifest, CheckKind, EndpointUrl, InstallId, Rollout,
    RolloutFraction, SourceError, SourceRequest, StaticJsonSource, UpdateAvailability,
    UpdateSource, evaluate,
};
use semver::Version;

/// An adapter of a shape Longhorn does not ship: a private host reached
/// through a token-authenticated proxy, addressing releases by identifier.
///
/// It exists to prove the trait accommodates a consumer's own source without
/// that source gaining any say over policy.
struct ProxiedPrivateSource {
    proxy: String,
    token: String,
}

impl UpdateSource for ProxiedPrivateSource {
    fn manifest_request(&self, channel: Channel) -> Result<SourceRequest, SourceError> {
        Ok(SourceRequest::new(EndpointUrl::new(format!(
            "{}/channels/{}",
            self.proxy,
            channel.as_str()
        ))?)
        .with_header("Authorization", format!("Bearer {}", self.token)))
    }

    fn artifact_request(&self, artifact: &Artifact) -> Result<SourceRequest, SourceError> {
        Ok(SourceRequest::new(EndpointUrl::new(&artifact.url)?)
            .with_header("Accept", "application/octet-stream")
            .with_header("Authorization", format!("Bearer {}", self.token)))
    }
}

fn version(value: &str) -> Version {
    Version::parse(value).unwrap()
}

#[test]
fn a_consumer_adapter_supplies_its_own_headers() {
    let source = ProxiedPrivateSource {
        proxy: "https://updates.example.com".into(),
        token: "secret".into(),
    };

    let request = source.manifest_request(Channel::Beta).unwrap();

    assert_eq!(
        request.url.as_str(),
        "https://updates.example.com/channels/beta"
    );
    assert_eq!(
        request.headers,
        vec![("Authorization".to_owned(), "Bearer secret".to_owned())]
    );
}

#[test]
fn a_consumer_adapter_inherits_policy_with_no_extra_wiring() {
    // The point of the trait split: an adapter says where the manifest lives
    // and nothing more. Rollout, the mandatory floor, and channel semantics
    // are applied identically whatever produced the manifest.
    let rollout = Rollout::new(RolloutFraction::new(0.5).unwrap(), "1.3.0");
    let excluded = (0..1_000)
        .map(|index| InstallId::new(format!("install-{index}")).unwrap())
        .find(|candidate| !rollout.includes(candidate))
        .expect("a half rollout must exclude someone");

    let manifest =
        ChannelManifest::new(Channel::Production, version("1.3.0")).with_rollout(rollout);
    let build = BuildIdentity::new(Channel::Production, version("1.2.9"));

    assert_eq!(
        evaluate(&build, &manifest, &excluded, CheckKind::Automatic),
        UpdateAvailability::WithheldByRollout {
            version: version("1.3.0")
        }
    );
}

#[test]
fn an_adapter_cannot_downgrade_transport_security() {
    // An adapter has no authority to weaken policy. Composing a plain-HTTP
    // URL fails at the adapter boundary rather than reaching a fetch.
    struct Insecure;

    impl UpdateSource for Insecure {
        fn manifest_request(&self, _channel: Channel) -> Result<SourceRequest, SourceError> {
            Ok(SourceRequest::new(EndpointUrl::new(
                "http://updates.example.com/production.json",
            )?))
        }
    }

    assert!(matches!(
        Insecure.manifest_request(Channel::Production),
        Err(SourceError::Url(_))
    ));
}

#[test]
fn the_shipped_adapters_agree_on_channel_naming() {
    // Channel names appear in composed URLs, so a divergence between
    // adapters would surface as a 404 on one host and not another.
    let static_source = StaticJsonSource::new("https://updates.example.com");

    for channel in Channel::ALL {
        let url = static_source.manifest_request(channel).unwrap();
        assert!(
            url.url
                .as_str()
                .ends_with(&format!("/{}.json", channel.as_str())),
            "{channel} composed {}",
            url.url
        );
    }
}
