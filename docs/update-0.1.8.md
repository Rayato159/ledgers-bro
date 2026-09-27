# Update to 0.1.8

This release adds complete AI batch review with inline editing and one final save, fixes the quick-entry composer layout, and adds a seven-day daily-expense chart to Overview.

## Install

Download the Windows MSI/portable ZIP or the Android test APK for your architecture from the GitHub release. Back up your local ledger before updating. Close the app before replacing a Windows portable installation; keep its assets, OCR and license folders together. Android test APKs retain the existing package identity and signing key.

No database migration or reset is required for these presentation/report additions. Existing users, journals, settings and installed AI models remain local. The installer does not include private data or model weights.

## Daily spending

Overview displays the last seven calendar days ending today. Zero-expense days still appear. Select a bar with a mouse, touch or keyboard to see that day's exact amount, transaction count and entries. Dates follow the ledger's current day, and the range can cross a month or year boundary.

Only active Expense entries are counted. Income, opening balances, inter-account transfers (including card repayments), reversed entries and unpaid recurring plans are excluded. Actual paid expenses remain included. Amounts are summed in integer minor units; bar heights are presentation only.

## Composer

Transaction review now opens in a wider dialog with one card per entry. Each card shows its description, amount, account, date and category. Check a card to approve it, or edit and save its draft inline. One final confirmation validates and records the entire batch atomically; saving an individual card does not write to the ledger. Failed saves keep the batch available for correction or retry.

Local Qwen uses a versioned batch proposal with up to eight distinct transactions in source order. Legacy alternative interpretations are still choices, never additional purchases. Missing or ungrounded model descriptions retain the user's original message for review instead of silently becoming blank. AI output remains an untrusted draft and requires human review.

Desktop controls share one toolbar. On narrow screens the model selector moves onto its own row. During a download, progress and cancellation use a separate full-width area so they cannot displace the input or hide the send button. Existing download persistence across tabs remains active.

Read BUILDINFO.txt for the checks performed on the release artifacts and remaining device-testing limits.
