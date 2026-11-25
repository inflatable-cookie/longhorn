Every proof that stages a Rust workspace takes its staging root from external
build scratch (`stagingDirectory`), not `os.tmpdir()`. A new static test fails
any such proof that touches `os.tmpdir()`.
