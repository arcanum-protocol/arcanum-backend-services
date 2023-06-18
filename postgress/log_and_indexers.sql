--CREATE TABLE IF NOT EXISTS multipool_assets
--(
--    address TEXT PRIMARY KEY,
--    ideal_share numeric NOT NULL DEFAULT '0',
--    quantity numeric NOT NULL DEFAULT '0',
--    price numeric NOT NULL DEFAULT '0'
--);
--
--CREATE TABLE IF NOT EXISTS indexers_height
--(
--    id numeric PRIMARY KEY,
--    block_height numeric NOT NULL
--);
--
--
--CREATE TABLE IF NOT EXISTS mp_to_asset
--(
--    mp_address TEXT PRIMARY KEY,
--    asset_id TEXT NOT NULL UNIQUE
--);

CREATE OR REPLACE PROCEDURE assemble_price(arg_index_id bigint) 
LANGUAGE plpgsql 
AS $$
DECLARE
    total_val BIGINT;
    ind indexes;
    new_price numeric;
    resolutions INT[] := '{60,180,300,900,1800,3600,43200,86400}';
    resol INT;
BEGIN 
    select * from indexes where id = arg_index_id into ind;
    IF ind.alg = 'mcap' THEN
        SELECT SUM(mcap) INTO total_val 
        FROM assets a 
        JOIN assets_to_indexes ati 
            ON a.id = ati.asset_id
        WHERE ati.index_id = arg_index_id;
        SELECT SUM(price * mcap) / total_val 
        INTO new_price
        FROM assets a 
        JOIN assets_to_indexes ati 
            ON a.id = ati.asset_id
        WHERE ati.index_id = arg_index_id;
    ELSIF ind.alg = 'revenue' THEN
        SELECT SUM(revenue) INTO total_val 
        FROM assets a 
        JOIN assets_to_indexes ati 
            ON a.id = ati.asset_id
        WHERE ati.index_id = arg_index_id;
        SELECT SUM(price * revenue) / total_val 
        INTO new_price
        FROM assets a 
        JOIN assets_to_indexes ati 
            ON a.id = ati.asset_id
        WHERE ati.index_id = arg_index_id;
    END IF;

    new_price = ROUND(new_price, 6);

    INSERT INTO prices(index_id, ts, price)
    VALUES(arg_index_id, (EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT,new_price);

    -- gen candles
    --                      1m 3m  5m  15m 30m  60m   12h   24h    
    FOREACH resol in array resolutions
    LOOP 
        INSERT INTO candles(index_id, ts, resolution, open, close, low, high)
        VALUES(
            arg_index_id,
            (EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT / resol * resol, 
            resol,
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
END 
$$;
