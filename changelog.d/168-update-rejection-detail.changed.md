Update rejections now include `detail` (nullable), and malformed artifacts
use `malformedArtifact` rather than `installFailed`. Handle the new code and
read `detail` when presenting installation failures. `UpdateCommandOutcome`
carries the same field. Common credential shapes are redacted from detail;
hosts must omit other secrets. Malformed and signature-rejected staged artifacts
are discarded; obtain a corrected artifact before trying again.
