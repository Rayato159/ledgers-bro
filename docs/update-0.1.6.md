# Update to 0.1.6

The signed-in username, **Edit user**, and **Switch user / Sign out** have moved to **More → Settings → General**. The account controls no longer occupy the top of every page. On phones, the actions stack with comfortable tap targets.

The existing username/password editor and sign-out behavior remain available from that settings row. These changes do not migrate or reset users, passwords, financial records, balances, recurring plans or receivables. Financial schema 10 and profile schema 2 are unchanged.

The notification bell now separates **Unread** from **Archived**. Opening a notification marks it read and archives its title, due date and amount immediately. **Read and archive all** clears the unread list. The last 256 read notifications remain available even after a bill is paid; archived amounts reflect the time they were read. History is stored separately for each user, and the dialog is wider on desktop while fitting mobile screens.

If running 0.1.5, use **Settings → App updates** after the release is published, or install the new package over the existing app. Windows MSI users keep the same installation and data directory. Android users should install the matching APK without uninstalling or clearing storage. Version 0.1.6 uses the existing Android test package and signing identity, with version code 1006.

Downloads include the Windows MSI and portable ZIP, plus ARM64 and x86_64 Android test APKs. Windows binaries remain unsigned; Android APKs are sideload/test builds, not Play Store packages. See `BUILDINFO.txt` and `SHA256SUMS.txt` in the release for the exact commit, validation and file hashes.

Verification uses only separate synthetic profiles and an isolated Android Studio emulator. No physical-phone or full Windows MSI installer-interaction test is claimed.
