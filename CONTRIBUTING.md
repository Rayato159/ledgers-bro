# Contributing

Bug reports and focused pull requests are welcome. Include steps to reproduce, platform details, and synthetic examples; never post real ledgers, receipts, credentials, or signing keys. Run the checks in [README](README.md) before submitting code changes. Contributions to project-owned code use the [MIT License](LICENSE); preserve third-party notices and attribution.

## Code conventions

## Layers

- `domain` owns accounting invariants. No Dioxus, SQLite, serde, OS clock, filesystem, network, or model runtime. Deterministic date/ID/error libraries are allowed.
- `application` owns use cases, input resolution, orchestration, projections, and ports. Never import adapters or UI.
- `infrastructure` implements persistence and OS adapters. Recheck shared policies against locked current data; do not duplicate formulas.
- `ui` handles interaction/presentation through `UiGateway`. The native host wires adapters. Components must not issue SQL, open files, call models, classify tax, or calculate balances.
- Introduce traits at effect/replaceable boundaries (`LedgerRepository`, `Clock`, `IdSource`, `UiGateway`), not one trait per struct. Avoid empty speculative services.

## Entities and value objects

- Entities have persistent identity (`Account`, `JournalEntry`). Use typed IDs for domain identity; names and current values are not identity. Complete snapshots can compare contents for rendering/tests.
- Value objects compare by value and validate construction (`Money`, `PositiveMoney`, `AccountName`, `Note`, `EntryDate`, typed IDs). Replace them rather than mutate into invalid states.
- A value object is not "static data". Changing a balance replaces `Money`, while the account remains the same entity.
- Keep invariant-bearing fields private. Never derive `Deserialize` on domain objects. DTOs must call validated constructors.

## Rust and accounting

- Edition 2024. Standard Rust naming; Dioxus `#[component]` functions use framework conventions.
- Run `cargo fmt` and Clippy with `-D warnings`. Workspace lints forbid unsafe code and deny production `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, and debug macros.
- Test fixture files may explicitly allow `expect`/`panic`. Do not suppress production lints globally.
- Typed errors with `Result`. Do not disclose SQL, statements, raw model outputs, or financial notes in logs/error messages.
- Supported currencies use bounded `i64` minor units (two decimal places; THB uses satang). A ledger has one immutable currency after its first account, bill, or receivable. Accumulate with a wider integer and validate. No floating-point accounting; chart geometry may use floats derived from final amounts.
- Calendar dates are Gregorian internally. Relative phrases resolve on submission. Never guess account, amount, category, or tax eligibility.
- Use closed enums for mutually exclusive transaction types. Keep persistence/model DTOs outside the domain.

## Persistence and effects

- Journals are immutable and balanced. Cancellation adds a reversal on the original effective date, preserving UTC recorded timestamps.
- Account + opening entry + postings, or both transfer sides, commit atomically. Recheck policies inside `BEGIN IMMEDIATE`.
- Idempotency identifies a logical submission. Identical purchases with different submission IDs must both count.
- Run database/model/file I/O outside the Dioxus thread, with bounded work queues.
- Versioned forward migrations. Refuse corrupt/foreign/newer databases; never replace a database to recover from an error.
- Test expected outcomes and rollback independently, not only serialization round trips. Use synthetic test data and isolated directories.

## Before marking a change complete

1. Identify the invariant/use case and owning layer.
2. Test changed accounting, interpretation, persistence, or tax behavior and relevant boundaries.
3. Run format, lint, tests, build, and UI interaction checks as relevant.
4. Keep implemented/planned status explicit. Passing tests is not proof of Android support, encryption, tax compliance, or Store approval.
5. Review dependency/license and privacy requirements before release; tax rule year is separate from app version.
