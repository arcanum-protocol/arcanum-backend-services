CREATE TABLE IF NOT EXISTS assets
(
    id BIGSERIAL PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    symbol TEXT NOT NULL UNIQUE,
    coingecko_id TEXT NOT NULL UNIQUE,
    defilama_id TEXT NULL UNIQUE,
    price numeric NULL,
    revenue numeric NULL,
    mcap numeric NULL,
    volume_24h numeric NULL,
    logo text NULL,
    price_change_24h numeric NULL
);

CREATE TABLE IF NOT EXISTS multipools
(
    id BIGSERIAL PRIMARY KEY NOT NULL,

    block_height numeric NOT NULL

    name TEXT NOT NULL UNIQUE,
    symbol TEXT NOT NULL UNIQUE,
    description TEXT NULL,

    rpc_url TEXT NOT NULL,
    chain TEXT NOT NULL,
    address TEXT NOT NULL UNIQUE,

    total_supply numeric NOT NULL DEFAULT '0',
    change_24h numeric NULL,
    low_24h numeric NULL,
    high_24h numeric NULL,
    current_price numeric NULL
);

CREATE TABLE IF NOT EXISTS multipool_assets
(
    asset_symbol TEXT NOT NULL,
    asset_address TEXT NOT NULL,
    multipool_address TEXT NOT NULL,
    asset_id BIGINT REFERENCES assets(id),
    ideal_share numeric NOT NULL DEFAULT '0',
    quantity numeric NOT NULL DEFAULT '0',
    chain_price numeric NOT NULL DEFAULT '0',
    CONSTRAINT asset_address_multipool_address_pkey PRIMARY KEY (multipool_address, asset_address)
);

CREATE TABLE IF NOT EXISTS candles
(
    index_id BIGINT NOT NULL REFERENCES indexes(id),
    ts bigint NOT NULL,
    resolution int NOT NULL,
    open numeric NOT NULL,
    close numeric NOT NULL,
    low numeric NOT NULL,
    high numeric NOT NULL,
    CONSTRAINT candles_price_pkey PRIMARY KEY (index_id, ts, resolution)
);

CREATE OR REPLACE PROCEDURE assemble_price(arg_index_id bigint) 
LANGUAGE plpgsql 
AS $$
DECLARE
    var_multipool_addresses VARCHAR[];
    var_multipool_address VARCHAR;
    var_total_supply numeric;
    new_price numeric;

    --                     1m 3m  5m  15m 30m  60m   12h   24h    
    var_resolutions INT[] := '{60,180,300,900,1800,3600,43200,86400}';
    var_resol INT;
BEGIN 

    SELECT 
        ARRAY_AGG(address) INTO var_multipools
    FROM multipools;

    FOREACH var_multipool_address in array var_multipool_addresses
    LOOP 
        SELECT total_supply INTO var_total_supply
        FROM multipools m
        WHERE m.address = var_multipool_address;
        SELECT 
            SUM(ma.quantity * a.price) / var_total_suppy 
            INTO new_price 
        FROM multipool_assets ma
        JOIN assets a 
            ON ma.asset_symbol = a.symbol
        WHERE ma.multipool_address = var_multipool_addresses;
        new_price = ROUND(new_price, 6);

        -- gen candles
        FOREACH var_resol in array var_resolutions
        LOOP 
            INSERT INTO candles(index_id, ts, resolution, open, close, low, high)
            VALUES(
                arg_index_id,
                (EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT / var_resol * var_resol, 
                var_resol,
                new_price,
                new_price,
                new_price,
                new_price
                )
            ON CONFLICT (index_id, ts, resolution) DO UPDATE SET
                close = new_price,
                low = least(candles.low, new_price),
                high = greatest(candles.high, new_price);
        END LOOP;
    END LOOP;
END 
$$;
