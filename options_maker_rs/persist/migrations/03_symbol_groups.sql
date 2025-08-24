CREATE TABLE symbol_groups
(
    sg_id      INTEGER      NOT NULL PRIMARY KEY AUTOINCREMENT,
    symbol     VARCHAR(16)  NOT NULL,
    group_name VARCHAR(256) NOT NULL,
    unique (symbol, group_name)
);
