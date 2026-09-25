# Local users and login

Each local user has a separate ledger. Create or select a user and enter a password before opening it. Rename the user, change the password, switch users or sign out under **Settings → General**. The header only identifies the current user.

**Remember me for 7 days** is off by default. Its expiry is measured from password confirmation; reopening the app does not extend it. Expiry requires a password on the next automatic login rather than terminating work already open. One user can be remembered per data directory.

Signing out, switching users, changing a password or logging in without Remember me revokes the previous token. Renaming alone preserves its expiry. Missing or corrupt token files, or a clock moving behind the last login time, return to login without deleting the ledger.

Usernames contain 1–40 characters and are unique without case sensitivity. Passwords require at least 8 characters and at most 256 bytes. Credential changes require the current password; leave the new password blank to rename only. Argon2id hashes use random salts. Five failed attempts lock that user for 30 seconds, including across app restarts. Old-session work is rejected after sign-out.

The native host stores a random 256-bit remembered-login token. The profile database keeps its SHA-256 hash and timestamps. The token is not stored in WebView/localStorage, exposed to the UI or included in CSV or `.lbro` backups. There is no JWT server or cloud account.

## Files and migration

- `user-profiles.sqlite3`: users, password hashes and login state.
- `remembered-login.token`: device-local automatic-login credential; never share or include in a backup.
- `ledger.sqlite3`: the legacy ledger, retained in place when upgrading from the original single-user app.
- `ledger-<opaque UUID>.sqlite3`: ledgers for newly created users; usernames are not used as paths.

The legacy profile requires initial password setup by the device owner. There is no default password. Installing over the same Windows user or Android package retains the data directory.

This login does not encrypt SQLite or protect files from someone with access to the same operating-system account. There is no email recovery or password retrieval. Keep [ledger backups](data-transfer.md) so data can be restored into a new empty user if necessary.
