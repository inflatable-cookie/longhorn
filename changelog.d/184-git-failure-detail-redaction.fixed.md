Every Git failure detail goes through the caller's redactor, including a
missing-folder failure. The invocation secret is redacted under the caller's own
rules instead of forcing the common ruleset.
