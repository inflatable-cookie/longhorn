Tooling now says what the hosted clients job runs. `effigy check:ci-static-parity`
fails when that job's TypeScript steps drift from the TypeScript family of
`qa`, and it confirms that `release.yml` still runs `effigy qa` for the gates
the job omits. `effigy check:markers` keeps the critical attention-marker tier
(`BUG`, `remove before release`) as a whole-word, case-sensitive gate over
tracked files. The retired `[scan.attention_markers]` and
`[scan.generated_in_src]` tables are removed from `effigy.toml`. The history
composition proof section names the live route, `effigy proof:artifacts`, in
place of two selectors that did not exist. No public API changed.
