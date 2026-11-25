Tooling now says what the hosted clients job runs. `effigy check:ci-static-parity`
fails when that job's TypeScript steps drift from the TypeScript family of
`qa`, and it confirms that `release.yml` still runs `effigy qa` for the gates
the job omits. It counts only unconditional workflow commands, in the clients
job and in any job of `release.yml`, so a guarded step or job, or an `echo`,
cannot satisfy it.
