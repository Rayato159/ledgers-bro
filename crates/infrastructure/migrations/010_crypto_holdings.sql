ALTER TABLE accounts ADD COLUMN btc_atoms INTEGER CHECK(btc_atoms IS NULL OR btc_atoms >= 0);
ALTER TABLE accounts ADD COLUMN sol_atoms INTEGER CHECK(
    (btc_atoms IS NULL AND sol_atoms IS NULL) OR
    (kind = 'crypto' AND btc_atoms IS NOT NULL AND sol_atoms IS NOT NULL AND sol_atoms >= 0)
);
CREATE TABLE crypto_holding_changes (
    id INTEGER PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    before_btc INTEGER,
    before_sol INTEGER,
    after_btc INTEGER NOT NULL,
    after_sol INTEGER NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE crypto_prices (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    btc_price INTEGER NOT NULL CHECK(btc_price > 0),
    btc_time INTEGER NOT NULL CHECK(btc_time >= 1230940800),
    sol_price INTEGER NOT NULL CHECK(sol_price > 0),
    sol_time INTEGER NOT NULL CHECK(sol_time >= 1230940800)
);
PRAGMA user_version = 10;
