The branch distance parser now reads ahead and behind counts only from
Git's trailing bracketed status summary, so branch names starting with
`ahead`, `behind` or `gone` can no longer mask the real fields.
