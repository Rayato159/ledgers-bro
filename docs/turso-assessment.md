# Turso assessment for Ledgers Bro

Current application decision: keep SQLite. This is the assessment recorded on 2026-09-25; it is not a live statement about Turso releases after that review.

The requested repository is the Rust Turso Database engine, not simply Turso's hosted service or the older libSQL fork. Its maintainers report production deployments and features including concurrent writes, CDC and native async I/O. They also say it has not reached 1.0 and is not yet fully SQLite compatible. See the [project FAQ](https://github.com/tursodatabase/turso#faq) and [compatibility table](https://github.com/tursodatabase/turso/blob/main/COMPAT.md), reviewed on 2026-09-25.

This application serializes ledger work through a worker and uses `rusqlite` with bundled SQLite, WAL, immediate transactions and foreign-key constraints. Backup validation in `crates/infrastructure/src/backup.rs` also executes `PRAGMA foreign_key_check`, which Turso's compatibility table listed as unsupported at that review. File-format compatibility alone does not validate the application's migration, restore and accounting behavior.

The repeated in-memory scans and large rendered lists found during this update can be improved without changing engines. The application indexes monthly settlement lookup, renders transaction history in pages of 20, adds a year filter and defers inactive read-only panels. The complete ledger is still loaded into application memory; this is not database-level pagination or a claim of constant memory usage.

No head-to-head Turso benchmark was performed. There is therefore no measured basis to claim that replacing SQLite would make this application faster. A future evaluation should use only generated fixtures, run the complete accounting/backup/migration suite against both engines, benchmark large ledgers on Windows and Android, and verify interruption/recovery behavior before proposing a migration of user data.
