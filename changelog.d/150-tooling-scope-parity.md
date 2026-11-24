Tooling now says what the hosted clients job runs. `effigy check:ci-static-parity`
fails when that job's TypeScript steps drift from the TypeScript family of
`qa`, and it confirms that `release.yml` still runs `effigy qa` for the gates
the job omits. It counts only unconditional workflow commands, in the clients
job and in any job of `release.yml`, so a guarded step or job, or an `echo`,
cannot satisfy it. `effigy check:markers` keeps the critical attention-marker
tier (`BUG`, `remove before release`) as a whole-word, case-sensitive gate over
tracked files. `effigy check:generated-sources` ports the retired generated-in-src
rules to the same source roots: generated path components, generated file names,
and generated header markers, with the bindings output and generator sources
allowed. Every file in a generated directory must be named by a generator. The retired `[scan.attention_markers]`
and `[scan.generated_in_src]` tables are removed from `effigy.toml`. The history
composition proof section names the live route, `effigy proof:artifacts`, in
place of two selectors that did not exist. No public API changed.
