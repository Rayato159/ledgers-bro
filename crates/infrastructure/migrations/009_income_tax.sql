-- Existing journal JSON remains valid: absent tax evidence means not selected.
-- Version gate prevents older readers silently discarding the new evidence.
PRAGMA user_version=9;
