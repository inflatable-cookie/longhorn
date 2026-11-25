A signed update archive that will not decompress or unpack — truncated
mid-entry, or cut inside the deflate body — is now refused as
`MalformedArtifact` instead of a retryable `Failed`. Extraction classifies the
archive itself, so a short stream and a short write are no longer the same
failure: malformed bytes are discarded with their staged copy rather than
replayed on every retry.
