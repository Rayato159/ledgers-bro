ALTER TABLE recurring_expenses ADD COLUMN installments INTEGER
    CHECK(installments IS NULL OR installments BETWEEN 1 AND 1200);
PRAGMA user_version = 3;
