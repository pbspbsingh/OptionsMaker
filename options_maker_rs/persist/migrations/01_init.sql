CREATE TABLE symbols
(
    symbol      VARCHAR(16)  NOT NULL PRIMARY KEY,
    exchange    VARCHAR(255) NOT NULL,
    asset_type  VARCHAR(255) NOT NULL,
    description VARCHAR(255) NOT NULL,
    cusip       VARCHAR(255),
    fundamental JSONB        NOT NULL,
    created_at  DATETIME     NOT NULL
);

CREATE TABLE prices
(
    price_id INTEGER     NOT NULL PRIMARY KEY AUTOINCREMENT,
    symbol   VARCHAR(16) NOT NULL,
    ts       DATETIME    NOT NULL,
    open     REAL        NOT NULL,
    low      REAL        NOT NULL,
    high     REAL        NOT NULL,
    close    REAL        NOT NULL,
    volume   INTEGER     NOT NULL,
    UNIQUE (symbol, ts)
);