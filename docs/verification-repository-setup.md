# Repository preparation — 21 September 2026

- Removed unused human-mascot/room copies and superseded concept PNGs after checking active Rust asset includes and build scripts. Current tanuki art, fonts, launcher icon source, and license notices remain.
- Removed temporary screenshots/logs and obsolete downloaded font/decoder copies. Historical design documents identify their superseded status; remaining relative Markdown links resolve.
- Excluded local databases, downloaded tools/models, editor settings, environment files, signing material, and build outputs from Git. A pattern scan of candidate source files found no matching credentials; this is not a full security audit.
- `scripts/check.ps1` passed: Rust formatting, workspace Clippy with warnings denied, and 94 tests. Two runtime-dependent OCR tests are ignored by the default suite.
- Ran those two OCR integration tests separately against the installed local runtime: both passed, including HEIC/JPEG, rotated HEIC, and reviewable receipt totals. Total executed tests: 96 passed.
- Retained the existing x86_64 and ARM64 debug APKs locally under `target/android`; they are not committed. Temporary compiler and packaging directories can be regenerated with the documented build commands.
- Personal/test ledger databases and installed OCR/LLM runtimes were preserved locally. This cleanup does not establish physical-device compatibility, complete tax support, or Store readiness.
