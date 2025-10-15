CREATE TABLE scanned_symbols (
    symbol VARCHAR(16) NOT NULL PRIMARY KEY,
    exchange VARCHAR(255) NOT NULL,
    sector VARCHAR(255) NOT NULL,
    industry VARCHAR(255) NOT NULL,
    price_changes JSONB NOT NULL,
    updated DATETIME NOT NULL
);

CREATE TABLE fudamentals (
    fid INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    symbol VARCHAR(16) NOT NULL,
    ts DATE NOT NULL,
    info TEXT NOT NULL,
    UNIQUE (symbol, ts)
);