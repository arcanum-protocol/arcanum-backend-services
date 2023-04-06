CREATE TABLE IF NOT EXISTS multipool_assets
(
    address TEXT PRIMARY KEY,
    ideal_share numeric NOT NULL DEFAULT '0',
    quantity numeric NOT NULL DEFAULT '0',
    price numeric NOT NULL DEFAULT '0'
);

CREATE TABLE IF NOT EXISTS indexers_height
(
    id numeric PRIMARY KEY,
    block_height numeric NOT NULL
);


CREATE TABLE IF NOT EXISTS mp_to_asset
(
    mp_address TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL UNIQUE
);

