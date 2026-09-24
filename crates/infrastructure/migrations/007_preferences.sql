CREATE TABLE user_preferences (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    english INTEGER NOT NULL CHECK (english IN (0,1)),
    dark INTEGER NOT NULL CHECK (dark IN (0,1)),
    primary_color INTEGER NOT NULL CHECK (primary_color BETWEEN 0 AND 16777215),
    gradient INTEGER NOT NULL CHECK (gradient IN (0,1))
) STRICT;
INSERT INTO user_preferences VALUES (1, 0, 0, 12427519, 1);
PRAGMA user_version=7;
