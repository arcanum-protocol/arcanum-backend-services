CREATE TABLE IF NOT EXISTS multipool
(
    address TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    assets TEXT[],
    curent_share numeric NOT NULL,
    ideal_share numeric NOT NULL,
    assets_amount numeric NOT NULL,
);

INSERT INTO multipool(
    address, 
    name, 
    assets, 
    current_share, 
    ideal_share, 
    assets_amount
) VALUES (
    '0x8EFa3E7bE538B07F3a80705E0d454384d0CbccF1',
    'basic',
    ARRAY ['----', '----'],
    0,
    0,
    0
);


CREATE TABLE IF NOT EXISTS indexers_height
(
    id numeric PRIMARY KEY,
    block_height numeric NOT NULL
);