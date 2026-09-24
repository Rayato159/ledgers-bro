CREATE TABLE ledger_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    currency TEXT NOT NULL CHECK (currency IN ('THB','USD','EUR','GBP','AUD','CAD','SGD','CNY')),
    thai_tax_enabled INTEGER NOT NULL CHECK (thai_tax_enabled IN (0,1) AND (thai_tax_enabled=0 OR currency='THB')),
    currency_locked INTEGER NOT NULL CHECK (currency_locked IN (0,1))
) STRICT;
INSERT INTO ledger_settings VALUES (1, 'THB', 1,
    EXISTS(SELECT 1 FROM accounts) OR EXISTS(SELECT 1 FROM journal_entries)
    OR EXISTS(SELECT 1 FROM recurring_expenses) OR EXISTS(SELECT 1 FROM receivables));
CREATE TRIGGER lock_currency_accounts AFTER INSERT ON accounts BEGIN
    UPDATE ledger_settings SET currency_locked = 1 WHERE id = 1;
END;
CREATE TRIGGER lock_currency_recurring AFTER INSERT ON recurring_expenses BEGIN
    UPDATE ledger_settings SET currency_locked = 1 WHERE id = 1;
END;
CREATE TRIGGER lock_currency_receivables AFTER INSERT ON receivables BEGIN
    UPDATE ledger_settings SET currency_locked = 1 WHERE id = 1;
END;
PRAGMA user_version=6;
