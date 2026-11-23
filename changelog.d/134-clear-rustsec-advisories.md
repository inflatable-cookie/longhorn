### Breaking
- **SQLite engine: rusqlite 0.40.2 to 0.41.1.** libsqlite3-sys moves 0.38.2 to 0.39.1, which clears RUSTSEC-2026-0459 (SQLite CVE-2026-119375). `bundled` and `backup` stay on. rusqlite is a dev-dependency of `longhorn-config`, so no shipped Longhorn code or Rust signature changes; consumers see the move only through the lock. A consumer that depends on rusqlite 0.40.x itself must move to 0.41.x in the same update, because libsqlite3-sys declares `links = "sqlite3"` and cargo resolves one copy per graph. Tom's decision 28232532 (2026-11-23) takes this as a Breaking change.

### Security
- **Clear RustSec advisories in the lock (RUSTSEC-2026-0457, -0459, -0461, -0462, -0486, -0592, -0594).** `rmcp` moves to 3.9.0, `cookie` to 0.18.3, `rustix` to 1.1.6, `url` to 2.5.9, and `libsqlite3-sys` to 0.39.1 through rusqlite 0.41.1. The chrono unmaintained advisory (RUSTSEC-2026-0543) is allowed in `deny.toml` because no safe upgrade exists.
