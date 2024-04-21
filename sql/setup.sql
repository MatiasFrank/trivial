CREATE TABLE IF NOT EXISTS questions (
    id INTEGER PRIMARY KEY,
    factory TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_answered_at INTEGER,
    probability REAL NOT NULL,
    num_correct INTEGER NOT NULL,
    num_incorrect INTEGER NOT NULL,
    data BLOB NOT NULL,
    UNIQUE(factory, name)
);
CREATE INDEX IF NOT EXISTS index_questions ON questions(factory, name);

CREATE TABLE IF NOT EXISTS answers (
    id INTEGER PRIMARY KEY,
    question_id INTEGER,
    time INTEGER,
    correct INTEGER
);
CREATE INDEX IF NOT EXISTS index_answers ON answers(question_id, time);

CREATE TABLE IF NOT EXISTS question_sets (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    question_id INTEGER NOT NULL,
    UNIQUE(name, question_id)
);

CREATE TABLE IF NOT EXISTS question_factories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    factory_type TEXT NOT NULL,
    data BLOB NOT NULL,
    UNIQUE(name)
);
