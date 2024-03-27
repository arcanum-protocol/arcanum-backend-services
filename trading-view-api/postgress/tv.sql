CREATE TABLE IF NOT EXISTS multipools
(
    multipool_id TEXT PRIMARY KEY NOT NULL,
    change_24h numeric NOT NULL DEFAULT '0',
    low_24h numeric NOT NULL DEFAULT '0',
    high_24h numeric NOT NULL DEFAULT '0',
    current_price numeric NOT NULL DEFAULT '0'
);

CREATE TABLE IF NOT EXISTS candles
(
    multipool_id VARCHAR NOT NULL REFERENCES multipools(multipool_id),
    ts bigint NOT NULL,
    resolution int NOT NULL,
    open numeric NOT NULL,
    close numeric NOT NULL,
    low numeric NOT NULL,
    high numeric NOT NULL,
    CONSTRAINT candles_pkey PRIMARY KEY (multipool_id, ts, resolution)
);

CREATE OR REPLACE PROCEDURE assemble_stats(arg_multipool_id VARCHAR, new_price numeric) 
LANGUAGE plpgsql 
AS $$
DECLARE
    highest numeric;
    min_ts bigint;
    lowest numeric;
    earliest numeric;

    --                         1m 3m  5m  15m 30m  60m   12h   24h    
    var_resolutions INT[] := '{60,180,300,900,1800,3600,43200,86400}';
    var_resol INT;
BEGIN 

        new_price = ROUND(new_price, 6);

        IF (select multipool_id from multipools where multipool_id=arg_multipool_id limit 1) IS NULL THEN
            insert into multipools(multipool_id) values (arg_multipool_id);
        END IF;

        -- gen candles
        FOREACH var_resol in array var_resolutions
        LOOP 
            INSERT INTO candles(multipool_id, ts, resolution, open, close, low, high)
            VALUES(
                arg_multipool_id,
                (EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT / var_resol * var_resol, 
                var_resol,
                new_price,
                new_price,
                new_price,
                new_price
                )
            ON CONFLICT (multipool_id, ts, resolution) DO UPDATE SET
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
            multipool_id=arg_multipool_id and 
            resolution=60 and
            ts > ((EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT - 86400) / 60 * 60;

        SELECT open 
        INTO earliest 
        FROM candles  
        WHERE 
            multipool_id=arg_multipool_id and 
            resolution=60 and
            ts > ((EXTRACT(epoch FROM CURRENT_TIMESTAMP::TIMESTAMP WITHOUT TIME ZONE))::BIGINT - 86400) / 60 * 60
        ORDER BY ts ASC
        LIMIT 1;

        UPDATE multipools 
        SET
            change_24h=CASE WHEN earliest <> 0 THEN ROUND((new_price-earliest) * '100'::numeric /earliest,6) ELSE '0'::numeric END,
            low_24h=lowest,
            high_24h=highest,
            current_price=new_price
        WHERE
            multipool_id=arg_multipool_id;
END 
$$;
