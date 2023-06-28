CREATE TABLE IF NOT EXISTS etf_assets
(
    asset_address TEXT NOT NULL,
    multipool_address TEXT NOT NULL,
    asset_id BIGINT REFERENCES assets(id),
    ideal_share numeric NOT NULL DEFAULT '0',
    quantity numeric NOT NULL DEFAULT '0',
    chain_price numeric NOT NULL DEFAULT '0',
    CONSTRAINT asset_address_multipool_address_pkey PRIMARY KEY (multipool_address, asset_address)
);

CREATE TABLE IF NOT EXISTS indexers_height
(
    id numeric PRIMARY KEY,
    block_height numeric NOT NULL
);

