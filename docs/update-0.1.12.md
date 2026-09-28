# Update 0.1.12

The Windows MSI removes the redundant **Uninstall LedgersBro** shortcut that caused a reported Warning 1909. The normal application shortcuts remain. To uninstall, use **Windows Settings → Apps → Installed apps → LedgersBro**. The MSI retains its upgrade code and legacy installation registry location.

The package publisher is **dancingwithmycode.com**, with a link to the publisher website. This is package metadata, not a code-signing certificate: Windows packages remain unsigned and Windows security prompts can still show an unknown publisher.

After updating, launch the new version once. It automatically removes recognized MSI/APK packages for its own version and older versions from the app's update cache. Locked files are retried on a later launch or update check. A newer downloaded package stays available if installation was cancelled or has not finished. The app does not scan Downloads, delete Windows Installer's system cache, follow cache links, or remove ledgers, exports, models or unrelated files.

Update from **Settings → App updates**, or install the matching release package over the existing app. Keep Android app storage intact. No database schema or financial-data changes are included.
