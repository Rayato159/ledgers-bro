# Updating to 0.1.4

Install over the existing application, using the same Windows data directory or the same Android package and signing identity. Do not uninstall the Android app or clear its storage. This version does not reset users, passwords, account balances, transactions, receivables or recurring plans. The ledger remains at schema 10 and the user registry at version 2. Export a full `.lbro` backup from Settings → Data before updating if you want an independent recovery copy.

## Changes

- Settings-style category tabs separate long Overview, Accounts, Transactions, Recurring Bills, Receivables, Tax and entry pages. History defaults to the current year and renders 20 entries per page; older years and all years remain available.
- Monthly bill cards truncate long names, have consistent actions and open full details in a dialog. Unpaid installments of stopped plans can be edited without restarting the plan after its original end date. Paid history remains intact.
- Adding a receivable opens a dialog with a spaced review/confirmation step. Returning to edit preserves the draft.
- Expense and credit-card pie charts share responsive labels and drilldowns. Other expenses are separated by description. Monthly bills have an unpaid-bill breakdown chart. Cash-flow details reconcile recorded expenses and optional pending bills without double counting payments.
- Redundant account/history banners, heading companions, the Overview prompt banner and top currency badge are removed. Crypto cards match other accounts, prices appear below content, and mixed Thai/English transaction text is larger.
- The AI panel offers Qwen3 0.6B through 14B, with hardware estimates and a confirmation before installation. See [model selection](local-model-selection.md).
- During January, ledgers with older history show a backup reminder. It links to full backup export and can be dismissed for the current app session. Nothing is automatically deleted or reset. Receivables and outstanding bills remain visible across years.

## Data and testing

UI verification uses a separate directory and the synthetic `ui_test_only` profile. Android verification uses a dedicated disposable emulator. User ledgers and personal backups must never be copied into these fixtures. Automated accounting tests create temporary databases.

Performance improvements cover monthly settlement lookup and bounded/deferred UI rendering. The application still loads the complete ledger into memory; database-level pagination remains a future option. [Turso was assessed](turso-assessment.md), and this release retains SQLite.

## Release verification

On 2026-09-25, 234 workspace tests passed; two environment-dependent OCR tests remained ignored. Formatting and strict Clippy checks passed. Windows verification opened the packaged executable with a synthetic data directory, navigated Transactions and opened a payment dialog. Responsive UI checks covered widths from 360 to 1440 CSS pixels, bill card sizing, modal behavior, chart drilldowns and mixed Thai/English text.

A fresh Android Studio x86_64 Emulator received the published 0.1.3 APK, a newly created test profile, a cash account, an expense and a recurring bill. Installing 0.1.4 over it preserved all financial and profile rows, with only the remembered-login last-use timestamp advancing. Tests then edited the bill from 250 to 260 THB and paid 260 THB, verifying that cash changed from 9,876.55 to 9,616.55 THB. Navigation, detail/edit/payment dialogs, charts and hardware checks also passed. Both ARM64 and x86_64 APK signatures and version metadata were checked; the ARM64 APK was not run on a physical phone.

Actual model downloads and activation were checked with Qwen3 1.7B on Windows and 0.6B in the Android APK. A Windows native CPU probe exercised eleven synthetic prompts: nine outputs passed the proposal contract, while two were rejected by validation. This is a runtime smoke check, not an accuracy benchmark. No 4B, 8B or 14B inference benchmark or complete MSI installer interaction is claimed.
