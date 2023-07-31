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

    block_height numeric NOT NULL,

    name TEXT NOT NULL UNIQUE,
    symbol TEXT NOT NULL UNIQUE,
    decimals INT NULL,
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
    asset_symbol TEXT NULL, -- should be set manually
    asset_address TEXT NOT NULL,
    multipool_address TEXT NOT NULL,
    ideal_share numeric NOT NULL DEFAULT '0',
    quantity numeric NOT NULL DEFAULT '0',
    decimals INT NULL,
    chain_price numeric NOT NULL DEFAULT '0',
    CONSTRAINT asset_address_multipool_address_pkey PRIMARY KEY (multipool_address, asset_address)
);

CREATE TABLE IF NOT EXISTS candles
(
    multipool_address VARCHAR NOT NULL REFERENCES multipools(address),
    ts bigint NOT NULL,
    resolution int NOT NULL,
    open numeric NOT NULL,
    close numeric NOT NULL,
    low numeric NOT NULL,
    high numeric NOT NULL,
    CONSTRAINT candles_price_pkey PRIMARY KEY (multipool_address, ts, resolution)
);

CREATE OR REPLACE PROCEDURE assemble_price() 
LANGUAGE plpgsql 
AS $$
DECLARE
    var_multipool_addresses VARCHAR[];
    var_multipool_address VARCHAR;
    var_total_supply numeric;
    new_price numeric;
    highest numeric;
    min_ts bigint;
    lowest numeric;
    earliest numeric;

    --                     1m 3m  5m  15m 30m  60m   12h   24h    
    var_resolutions INT[] := '{60,180,300,900,1800,3600,43200,86400}';
    var_resol INT;
BEGIN 

    SELECT 
        ARRAY_AGG(address) INTO var_multipool_addresses
    FROM multipools;

    FOREACH var_multipool_address in array var_multipool_addresses
    LOOP 
        SELECT total_supply / (10::numeric ^ decimals::numeric) INTO var_total_supply
        FROM multipools m
        WHERE m.address = var_multipool_address;
        IF var_total_supply != '0' THEN
            SELECT 
                SUM(ma.quantity * a.price / (10::numeric ^ ma.decimals::numeric)) / var_total_supply 
                INTO new_price 
            FROM multipool_assets ma
            JOIN assets a 
                ON ma.asset_symbol = a.symbol
            WHERE ma.multipool_address = var_multipool_address;
            new_price = ROUND(new_price, 6);

            -- gen candles
            FOREACH var_resol in array var_resolutions
            LOOP 
                INSERT INTO candles(multipool_address, ts, resolution, open, close, low, high)
                VALUES(
                    var_multipool_address,
                    (EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT / var_resol * var_resol, 
                    var_resol,
                    new_price,
                    new_price,
                    new_price,
                    new_price
                    )
                ON CONFLICT (multipool_address, ts, resolution) DO UPDATE SET
                    close = new_price,
                    low = least(candles.low, new_price),
                    high = greatest(candles.high, new_price);
            END LOOP;
            
            SELECT 
                MAX(high),
                MIN(low)
            INTO highest, lowest
            FROM 
                candles
            WHERE 
                multipool_address=var_multipool_address and 
                resolution=60 and
                ts > ((EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT - 86400) / 60 * 60;

            SELECT open 
            INTO earliest 
            FROM candles  
            WHERE 
                multipool_address=var_multipool_address and 
                resolution=60 and
                ts > ((EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT - 86400) / 60 * 60
            ORDER BY ts ASC
            LIMIT 1;

            UPDATE multipools 
            SET
                change_24h=ROUND((new_price-earliest) * '100'::numeric /earliest,6),
                low_24h=lowest,
                high_24h=highest,
                current_price=new_price
            WHERE
                address=var_multipool_address;

        END IF;
    END LOOP;
END 
$$;
