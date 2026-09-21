# Verification — 20 September 2026

Scope: Windows executable slice in this workspace. No Android emulator, physical Android, iPhone, Store submission, or legal tax-engine verification was performed.

## Automated checks

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` — passed.
- `cargo test --workspace --locked --offline` — 33 tests passed: 8 domain, 5 deterministic parser, 5 model boundary, 14 SQLite/use-case integration, 1 Dioxus lifecycle regression.
- `cargo build -p ledgers-bro --release --locked --offline` — passed, producing `target/release/ledgers-bro.exe`. This is an unsigned Windows executable, not a Store package.

Tests cover exact cents and overflow, leap/year boundaries, account identity/count, balanced journals, credit spending versus repayments, cancellation, whole-input parsing, unknown/ambiguous names, source-grounded model proposals, real-file reopen, idempotent retry versus a second real purchase, stale account state, transaction rollback after injected second-posting failure, rollback of opening/account creation, concurrent account #100, corruption/newer/foreign database refusal, and CSV formula neutralization.

## Native UI exercise

Used `agent-browser` via the debug build's WebView2 CDP endpoint, against synthetic data in `.data/ui-smoke`:

1. Created a cash account with an opening balance of THB 5,000. Opening did not count as income.
2. Parsed `กาแฟ 80`. Account and category remained unselected; selected cash and food, then reviewed the confirmation. In the final build, also verified the review button is disabled until both choices are provided and enabled after selecting them.
3. Saved and restarted the actual desktop application. Balance persisted as THB 4,920.
4. Saved `จ่าย 20 จาก เงินสด หมวด ของกินเล่น`. Dashboard refreshed to THB 4,900 with expenses THB 100.
5. Cancelled the THB 20 transaction with explicit confirmation. It remained visibly cancelled in history; balance returned to THB 4,920 and expenses THB 80.
6. Opened the tax page and verified it explicitly says calculation is unavailable, without a tax amount.
7. Set WebView viewport to 390 × 844. The document did not overflow horizontally; navigation and cancellation controls were usable. This is a responsive-layout check, **not an Android device/emulator test**.
8. No page JavaScript errors were reported in the inspected session. The native CSV save dialog was not automated; CSV content was tested through the application layer.

The save exercise exposed a real bug: closing the originating dialog/preview cancelled its scoped async task before the dashboard refreshed. Moved mutation tasks to the root scope and added `commit_refresh_survives_unmounting_the_originating_component`, which delays refresh until after the child unmounts.

Visual desktop screenshot was captured and inspected at `.preview/desktop.png`. Some subsequent captures timed out when the native window was not rendering foreground frames; those captures are not claimed as visual checks. Mobile checks here are DOM/interaction checks, not a captured Android screenshot.

Debug-only `--inspect-port` is for synthetic test data. It is rejected by release builds. Separate debug WebView profiles avoid sharing browser arguments across test instances. The ordinary application does not expose a debugging endpoint.

## Remaining gates

Encryption/key storage and backup/restore, model/OCR integration, platform bridges, real-device/lifecycle tests, accessibility/contrast and large-ledger performance, tax rule verification, privacy/license/dependency audit, signed packaging, and Store review remain open. The presence of passing checks is not a claim that these features are complete.
