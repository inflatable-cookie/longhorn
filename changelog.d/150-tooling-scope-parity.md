Tooling now says what the hosted clients job runs. `effigy check:ci-static-parity`
fails when that job's TypeScript steps drift from the TypeScript family of
`qa`, and it confirms that `release.yml` still runs `effigy qa` for the gates
the job omits. It counts only unconditional workflow commands, in the clients
job and in any job of `release.yml`, so a guarded step or job, or an `echo`,
cannot satisfy it.

The retired `scan attention-markers` and `scan generated-in-src` rules are ported
as `effigy check:markers` and `effigy check:generated-sources`. Each rule has a
synthetic fixture test, and both ports matched the stable scanners on one
fixture that covers every rule. `check:markers` fails on its critical tier
(`BUG`, `remove before release`). `check:generated-sources` covers the generated
header, source maps, minified assets, generated file names and generated path
components over the scanner's source roots, with the bindings output and
generator sources allowed.

The retired `[scan.attention_markers]` and `[scan.generated_in_src]` tables are
removed from `effigy.toml`. The history composition proof section names the live
route, `effigy proof:artifacts`, in place of two selectors that did not exist.
No public API changed.
