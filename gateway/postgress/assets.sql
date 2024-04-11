CREATE TABLE IF NOT EXISTS chains
(
    chain_id            INT     NOT NULL,

    router_address      TEXT    NOT NULL,
    multicall_address   TEXT    NOT NULL,
    farm_address        TEXT    NOT NULL,

    PRIMARY KEY (chain_id)
);

CREATE TABLE IF NOT EXISTS assets
(
    address     TEXT    NOT NULL,
    chain_id    INT     NOT NULL REFERENCES chains(chain_id),
    name        TEXT    NOT NULL,
    symbol      TEXT    NOT NULL,
    decimals    INT     NOT NULL,
    logo_url    TEXT    NOT NULL,
    twitter_url TEXT    NOT NULL,
    website_url TEXT    NOT NULL,
    description TEXT        NULL,

    PRIMARY KEY (chain_id, address)
);

CREATE TABLE IF NOT EXISTS weth_uniswap_pools
(
    address         TEXT    NOT NULL,
    chain_id        INT     NOT NULL REFERENCES chains(chain_id),

    asset_address   TEXT    NOT NULL,
    fee             INT     NOT NULL,

    FOREIGN KEY (chain_id, asset_address) REFERENCES assets(chain_id, address),
    PRIMARY KEY (chain_id, address)
);

CREATE TABLE IF NOT EXISTS silo_pools
(
    address             TEXT    NOT NULL,
    chain_id            INT     NOT NULL REFERENCES chains(chain_id),

    asset_address       TEXT    NOT NULL,
    base_asset_address  TEXT    NOT NULL,

    FOREIGN KEY (chain_id, asset_address) REFERENCES assets(chain_id, address),
    FOREIGN KEY (chain_id, base_asset_address) REFERENCES assets(chain_id, address),
    PRIMARY KEY (chain_id, address)
);

CREATE TABLE IF NOT EXISTS etfs
(
    address             TEXT    NOT NULL,
    chain_id            INT     NOT NULL REFERENCES chains(chain_id),

    cb_vault_address    TEXT    NOT NULL,
    verified            BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (chain_id, address)
);
