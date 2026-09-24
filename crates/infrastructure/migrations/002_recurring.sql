CREATE TABLE recurring_expenses (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 60),
    amount_minor INTEGER NOT NULL CHECK(amount_minor BETWEEN 1 AND 9000000000000),
    category TEXT NOT NULL,
    account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
    day INTEGER NOT NULL CHECK(day BETWEEN 1 AND 31),
    start_month TEXT NOT NULL,
    stopped_from TEXT
) STRICT;
CREATE TABLE recurring_settlements (
    recurring_id TEXT NOT NULL REFERENCES recurring_expenses(id),
    month TEXT NOT NULL,
    entry_id TEXT NOT NULL UNIQUE REFERENCES journal_entries(id) ON DELETE CASCADE,
    PRIMARY KEY(recurring_id, month)
) STRICT;
PRAGMA user_version = 2;
