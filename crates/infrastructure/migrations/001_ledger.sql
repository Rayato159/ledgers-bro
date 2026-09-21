CREATE TABLE accounts (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 60),
    name_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK(kind IN ('cash','bank','credit','crypto','investment')),
    archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1))
) STRICT;

-- Defense in depth for the domain's MAX_ACCOUNTS = 100, including archived rows.
CREATE TRIGGER accounts_limit BEFORE INSERT ON accounts
WHEN (SELECT COUNT(*) FROM accounts) >= 100
BEGIN SELECT RAISE(ABORT, 'account limit'); END;

CREATE TABLE journal_entries (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    submission_id TEXT NOT NULL UNIQUE,
    effective_date TEXT NOT NULL,
    note TEXT NOT NULL CHECK(length(note) <= 500),
    payload TEXT NOT NULL,
    reverses TEXT UNIQUE REFERENCES journal_entries(id),
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

CREATE TABLE postings (
    entry_id TEXT NOT NULL REFERENCES journal_entries(id),
    ordinal INTEGER NOT NULL CHECK(ordinal IN (0,1)),
    account_id TEXT REFERENCES accounts(id),
    system_book TEXT CHECK(system_book IN ('equity','income','expense')),
    amount_minor INTEGER NOT NULL CHECK(amount_minor BETWEEN -9000000000000 AND 9000000000000),
    CHECK ((account_id IS NOT NULL AND system_book IS NULL) OR (account_id IS NULL AND system_book IS NOT NULL)),
    PRIMARY KEY(entry_id, ordinal)
) STRICT;

CREATE INDEX postings_by_account ON postings(account_id);
CREATE INDEX entries_by_date ON journal_entries(effective_date, sequence);

PRAGMA application_id = 1279414863;
PRAGMA user_version = 1;
