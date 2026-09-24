CREATE TABLE receivables (
    id TEXT PRIMARY KEY NOT NULL,
    debtor TEXT NOT NULL CHECK(length(debtor) BETWEEN 1 AND 60),
    description TEXT NOT NULL CHECK(length(description) BETWEEN 1 AND 300),
    total_minor INTEGER NOT NULL CHECK(total_minor BETWEEN 1 AND 9000000000000),
    opened TEXT NOT NULL,
    start_month TEXT NOT NULL,
    day INTEGER CHECK(day IS NULL OR day BETWEEN 1 AND 31),
    installments INTEGER CHECK(installments IS NULL OR installments BETWEEN 1 AND 1200)
) STRICT;

-- Postings has no incoming foreign keys. Rebuild it transactionally to extend
-- the system-book CHECK; journal identities and accounting values stay intact.
CREATE TABLE postings_v4 (
    entry_id TEXT NOT NULL REFERENCES journal_entries(id),
    ordinal INTEGER NOT NULL CHECK(ordinal IN (0,1)),
    account_id TEXT REFERENCES accounts(id),
    system_book TEXT CHECK(system_book IN ('equity','income','expense','receivable')),
    amount_minor INTEGER NOT NULL CHECK(amount_minor BETWEEN -9000000000000 AND 9000000000000),
    CHECK ((account_id IS NOT NULL AND system_book IS NULL) OR (account_id IS NULL AND system_book IS NOT NULL)),
    PRIMARY KEY(entry_id, ordinal)
) STRICT;
INSERT INTO postings_v4 SELECT * FROM postings;
DROP TABLE postings;
ALTER TABLE postings_v4 RENAME TO postings;
CREATE INDEX postings_by_account ON postings(account_id);
PRAGMA user_version = 4;
