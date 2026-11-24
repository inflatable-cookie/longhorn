### Breaking

- Update rejections now include `detail` (nullable), and malformed artifacts
  use `malformedArtifact` rather than `installFailed`. Handle the new code and
  read `detail` when presenting installation failures. `UpdateCommandOutcome`
  carries the same field. Malformed staged artifacts are discarded; fetch a
  corrected artifact before trying again.
- Update availability adds `unavailableForTarget` for releases without an
  artifact for the current target. Handle `UpdateAvailability::UnavailableForTarget`
  and `UpdateAvailabilityProjection::UnavailableForTarget` in exhaustive matches;
  show the release as unavailable instead of offering install.

### Fixed

- Preserve active download progress when deferring and redact request secrets
  and verified artifact bytes from debug output.
