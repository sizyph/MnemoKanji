-- MnemoKanji seed schema (slice 1: kanji core + component graph + ordering).
-- Forward-compatible; later M1 slices add phonetic/vocab/sentence/actor/mnemonic tables.
-- See docs/03-DESIGN.md §2 and docs/08-DATASET.md.

PRAGMA foreign_keys = ON;

-- Build/provenance/attribution metadata (key/value).
CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- JLPT levels. ord = learning order (N5 first = 5..1 mapped to ord 1..5).
CREATE TABLE level (
    id          INTEGER PRIMARY KEY,
    jlpt        TEXT NOT NULL UNIQUE,        -- 'N5'..'N1'
    ord         INTEGER NOT NULL,            -- 1 = learned first (N5)
    kanji_count INTEGER NOT NULL DEFAULT 0
);

-- A teachable structural component (radical or sub-kanji primitive).
CREATE TABLE component (
    id        INTEGER PRIMARY KEY,
    glyph     TEXT NOT NULL UNIQUE,
    is_kanji  INTEGER NOT NULL DEFAULT 0     -- 1 if this component is itself a learned kanji
);

CREATE TABLE kanji (
    id              INTEGER PRIMARY KEY,
    glyph           TEXT NOT NULL UNIQUE,
    level_id        INTEGER NOT NULL REFERENCES level(id),
    stroke_count    INTEGER,
    freq            INTEGER,                 -- KANJIDIC freq rank (lower = more frequent); NULL if none
    primary_keyword TEXT,                    -- normalized first real meaning (no invented keywords)
    intro_rank      INTEGER                  -- within-level learning order (0-based)
);

-- on/kun readings for a kanji. is_dominant is set in the vocab slice (provisional 0 here).
CREATE TABLE reading (
    id          INTEGER PRIMARY KEY,
    kanji_id    INTEGER NOT NULL REFERENCES kanji(id),
    kind        TEXT NOT NULL,               -- 'on' | 'kun'
    reading     TEXT NOT NULL,               -- kana (kun may carry .okurigana / - prefix markers)
    is_dominant INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE meaning (
    id          INTEGER PRIMARY KEY,
    kanji_id    INTEGER NOT NULL REFERENCES kanji(id),
    gloss       TEXT NOT NULL,
    is_primary  INTEGER NOT NULL DEFAULT 0,
    sense_order INTEGER NOT NULL
);

-- Component-of edge. role = 'semantic' | 'phonetic' (phonetic set in the phonetic slice).
CREATE TABLE kanji_component (
    kanji_id     INTEGER NOT NULL REFERENCES kanji(id),
    component_id INTEGER NOT NULL REFERENCES component(id),
    role         TEXT NOT NULL DEFAULT 'semantic',
    PRIMARY KEY (kanji_id, component_id)
);

-- Authored content (slice 2): original MnemoKanji content from data/authored/*.json.
-- Component-actor registry: one persistent persona per component (docs/07 §1.1).
CREATE TABLE component_actor (
    component_id INTEGER PRIMARY KEY REFERENCES component(id),
    actor_name   TEXT NOT NULL,
    image        TEXT NOT NULL
);

-- Reading-actor registry: one persona per on'yomi, with vowel length (docs/07 §1.2).
CREATE TABLE reading_actor (
    reading      TEXT PRIMARY KEY,
    vowel_length TEXT NOT NULL,                 -- 'short' | 'long'
    actor_name   TEXT NOT NULL,
    note         TEXT
);

-- Per-kanji mnemonic story (generated + adversarially verified; docs/07 §2-3).
CREATE TABLE mnemonic (
    kanji_id          INTEGER PRIMARY KEY REFERENCES kanji(id),
    story             TEXT NOT NULL,
    reading_story     TEXT,
    reading_actor     TEXT,
    meaning_placement TEXT,                     -- 'start' | 'end'
    origin            TEXT NOT NULL DEFAULT 'generated',
    verified          INTEGER NOT NULL DEFAULT 0,
    issues            TEXT,                      -- JSON array of judge issues (if any)
    imageability      INTEGER,
    distinctiveness   INTEGER
);

-- In-context vocabulary (slice 3): common words using N5 kanji, from JMdict-common.
CREATE TABLE vocab (
    id      INTEGER PRIMARY KEY,
    surface TEXT NOT NULL,
    reading TEXT NOT NULL,
    gloss   TEXT NOT NULL,
    UNIQUE (surface, reading)
);

-- Which kanji a vocab word uses, and the kana that kanji contributes (per JmdictFurigana).
CREATE TABLE vocab_kanji (
    vocab_id        INTEGER NOT NULL REFERENCES vocab(id),
    kanji_id        INTEGER NOT NULL REFERENCES kanji(id),
    reading_in_word TEXT,                       -- e.g. 学 in 学校 -> 'がっ'
    PRIMARY KEY (vocab_id, kanji_id)
);

CREATE INDEX idx_vk_kanji      ON vocab_kanji(kanji_id);
CREATE INDEX idx_vk_vocab      ON vocab_kanji(vocab_id);
CREATE INDEX idx_reading_kanji ON reading(kanji_id);
CREATE INDEX idx_meaning_kanji ON meaning(kanji_id);
CREATE INDEX idx_kc_kanji      ON kanji_component(kanji_id);
CREATE INDEX idx_kc_component  ON kanji_component(component_id);
CREATE INDEX idx_kanji_level   ON kanji(level_id);
