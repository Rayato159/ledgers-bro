# Update to 0.1.10

This patch corrects the seven-day spending card in Overview. Content now has consistent padding on every edge, the heading and total are separated, and the selected-day summary has clearer spacing.

On narrow screens, date labels use separate day and month lines to keep all seven columns readable. Today's column uses a small marker; its full date and amount remain available through the day button and selected-day details.

The chart's calculations are unchanged. Transfers and cancelled entries remain excluded, and selecting a day still shows its recorded expense details. No database migration is required.

The recurring-plan editor now distinguishes changing terms from a chosen month from moving the start of an entirely unpaid plan. Moving a plan can change its starting month in either direction and retains its instalment count unless explicitly edited. A confirmation appears before saving; Cancel leaves storage untouched, and a successful save reloads the list. Plans with any payment history cannot be moved as a whole, and overlapping versions of the same named plan in the same account are rejected. Changing only the effective month in terms mode now shows an explanation instead of silently closing.

Install the Windows MSI or portable ZIP, or the Android test APK for your architecture. Back up your data first, then update over the existing installation without clearing storage. Installed data and models are retained. Personal data, test profiles, model weights and signing credentials are excluded from the packages.

See BUILDINFO.txt for completed verification and platform limits. SHA256SUMS.txt contains download checksums.
