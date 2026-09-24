CREATE TABLE prompt_submissions (
    id TEXT PRIMARY KEY NOT NULL,
    fingerprint TEXT NOT NULL CHECK(length(fingerprint)=64)
) STRICT;
PRAGMA user_version = 5;
