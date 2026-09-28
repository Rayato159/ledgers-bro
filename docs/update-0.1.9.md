# Update to 0.1.9

This release fixes update-card spacing, left-aligns the quick-entry model selector, and clarifies recurring-plan schedules and editing.

## UI changes

Monthly bills now open in a timeline, with recorded, unpaid and overdue statuses and a marker for today in the current month. Each marker sits on the bill's due date, not the actual payment date. Past and future months are labeled accordingly. Filters work in both timeline and list views; selecting a bill opens its details. The chart scrolls within a bounded area, including on narrow screens. Credit-card charges remain labeled as awaiting card repayment.

The update card now has padding on every edge, consistent spacing between its status and controls, and compact download, cancel and install buttons. Download progress uses the selected theme colors and a right-aligned percentage. Long package names wrap within the card.

The model selector follows the receipt, microphone and prompt-example controls on desktop. Only the send button uses the remaining space to stay on the right. On narrow screens, the model selector remains on its own left-aligned row. Model downloads keep their separate progress row.

Recurring plan summaries now show the configured payment day for both ongoing bills and fixed installments. The edit dialog explicitly selects the plan that was clicked, keeping the dropdown and editable details in sync. Changing to a month where that plan does not apply shows the selection prompt instead of silently displaying the first unrelated plan. Earlier periods and recorded payments keep their existing details.

## Install and data

Use the Windows MSI or portable ZIP, or the Android test APK matching your architecture. Back up your ledger before updating. Install over the existing app; do not clear Android storage or uninstall first. The Android package identity and signing certificate remain unchanged.

No database migration or financial behavior changes are included. Existing users, entries, settings and installed models remain local. Personal data, test profiles, signing keys and model weights are excluded from the release.

Read BUILDINFO.txt for artifact verification and device-testing limits. SHA256SUMS.txt lists the download checksums.
