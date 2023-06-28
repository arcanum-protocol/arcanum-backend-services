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

CREATE TYPE aggregation_algorithm AS ENUM ('mcap', 'revenue');

CREATE TABLE IF NOT EXISTS indexes
(
    id BIGSERIAL PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    symbol TEXT NOT NULL UNIQUE,
    description TEXT NULL,
    price_change_24h numeric NULL,
    mcap numeric NULL,
    alg aggregation_algorithm NOT NULL
);

CREATE TABLE IF NOT EXISTS assets_to_indexes
(
    id BIGSERIAL PRIMARY KEY NOT NULL,
    index_id BIGINT NOT NULL REFERENCES indexes(id),
    asset_id BIGINT NOT NULL REFERENCES assets(id)
);

CREATE TABLE IF NOT EXISTS prices
(
    index_id BIGINT NOT NULL REFERENCES indexes(id),
    ts bigint NOT NULL,
    price numeric NOT NULL,
    CONSTRAINT prices_price_pkey PRIMARY KEY (index_id, ts)
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


--insert into assets(name, coingecko_id, price, revenue, mcap) 
--values 
--('btc', 'btc', '24000', '10', '10'),
--('eth', 'eth', '18000', '10', '10'),
--('bnb', 'bnb', '300', '10', '10');
--
--insert into indexes(name, alg) 
--values 
--('test mcap', 'mcap'),
--('test revenue', 'revenue');
--
--insert into assets_to_indexes(index_id, asset_id) 
--values 
--(1,1),
--(1,2),
--(1,3),
--(2,1),
--(2,2),
--(2,3);
--
--call assemble_price(1);
--call assemble_price(2);
--

