### Breaking

- Update rejections now include `detail` (nullable), and malformed artifacts
  use `malformedArtifact` rather than `installFailed`. Handle the new code and
  read `detail` when presenting installation failures. `UpdateCommandOutcome`
  carries the same field. Common credential shapes are redacted from detail;
  hosts must omit other secrets. Malformed and signature-rejected staged artifacts
  are discarded; obtain a corrected artifact before trying again.
- `evaluate` now requires the installation target. Pass the target triple so
  policy can refuse offers with no matching artifact.
- Update availability adds `unavailableForTarget` for releases without an
  artifact for the current target. Handle `UpdateAvailability::UnavailableForTarget`
  and `UpdateAvailabilityProjection::UnavailableForTarget` in exhaustive matches;
  show the release as unavailable instead of offering install.

### Fixed

- Preserve active download progress when deferring and redact request secrets
  and verified artifact bytes from debug output.
