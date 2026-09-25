-- Existing cards remain explicitly unscheduled until their owner supplies terms.
ALTER TABLE accounts ADD COLUMN closing_day INTEGER
    CHECK (closing_day IS NULL OR (kind = 'credit' AND closing_day BETWEEN 1 AND 31));
ALTER TABLE accounts ADD COLUMN payment_day INTEGER
    CHECK ((payment_day IS NULL AND closing_day IS NULL)
        OR (payment_day IS NOT NULL AND payment_day BETWEEN 1 AND 31 AND closing_day IS NOT NULL));
PRAGMA user_version=8;
