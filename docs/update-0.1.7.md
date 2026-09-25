# Update to 0.1.7

This release fixes recurring-plan editing, model download state, and chart interaction.

## Recurring plans

Choose **Apply changes from (AD month)**, then **Plan to update**. If an old stopped plan does not apply in the selected month, explicitly choose the plan for that month. Selecting a different plan loads its terms for editing. Change the payment day and other terms, save, then confirm or cancel. Successful saves refresh the ledger automatically.

Earlier periods keep their previous terms. Months with payment history cannot be rewritten; choose a month after the latest recorded payment. Stopped plans are not silently resumed. If no plan applies in a month, choose another month or create a new plan.

## Model downloads

Switching tabs no longer cancels a download. Returning to the AI controls shows its current progress or result. Explicit cancellation and closing the app session still stop the operation. Resuming a partially downloaded file after restarting the app is not implemented.

Use **Settings → On-device AI** to remove installed models. On Windows, their default location is `%LOCALAPPDATA%\Dancing With My Code\Ledgers Bro\data\models`. Delete only model files when removing a model; the parent data directory also holds financial records.

## Charts

Hover a pie-chart segment to see its full label, currency amount, and percentage. Keyboard focus shows the same information; Enter or Space opens details. Tapping a segment opens its details on touch devices. All pie-chart views use the same interaction, including very small segments near the top of the circle.

## Install and retain data

The README and active documentation are now in English and describe the current app. Superseded specifications, old verification diaries, obsolete design studies and old documentation screenshots have been removed. Current uncle-crab masters and launcher resources are retained.

Use **Settings → App updates**, or install the matching package over the existing app. Back up first. Do not uninstall the Android app or clear its storage. The financial ledger remains at schema 10 and the user registry at schema 2.

The release includes unsigned Windows x64 MSI and portable ZIP packages, plus debug-signed ARM64 and x86_64 Android test APKs. Android retains package `com.dancingwithmycode.ledgersbro.test` and its existing signing certificate, with version code 1007. These are sideload packages, not Play Store builds.

See the release's `BUILDINFO.txt` and `SHA256SUMS.txt` for the exact source commit, validation performed, and file hashes. Verification uses isolated synthetic ledgers and an Android Studio emulator; it does not claim a physical-phone test or a full Windows MSI installation test.
