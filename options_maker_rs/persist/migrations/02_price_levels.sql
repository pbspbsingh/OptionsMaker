CREATE TABLE price_levels
(
    symbol       VARCHAR(16) NOT NULL PRIMARY KEY,
    price_levels TEXT        NOT NULL,
    updated_at   DATETIME    NOT NULL
);
