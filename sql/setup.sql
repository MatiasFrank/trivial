CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY,
    question_set TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_answered_at INTEGER,
    probability REAL NOT NULL,
    num_correct INTEGER NOT NULL,
    num_incorrect INTEGER NOT NULL,
    data BLOB NOT NULL,
    UNIQUE(question_set, name)
);

CREATE TABLE IF NOT EXISTS answers (
    id INTEGER PRIMARY KEY,
    question_id INTEGER,
    time INTEGER,
    correct INTEGER
);

CREATE TABLE IF NOT EXISTS question_sets (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    set_type TEXT NOT NULL,
    data BLOB NOT NULL,
    UNIQUE(name)
);
