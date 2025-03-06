CREATE TABLE IF NOT EXISTS chains (
    chain_id BIGSERIAL PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS multipools 
(
    chain_id                BIGSERIAL   NOT NULL,
    multipool_id            BIGSERIAL   NOT NULL,
    multipool_address       TEXT        NOT NULL, 

    description             TEXT        NOT NULL,
    name                    TEXT        NOT NULL,
    symbol                  TEXT        NOT NULL,

    is_verified             BOOLEAN     NOT NULL DEFAULT false,

    total_supply            NUMERIC     NOT NULL,

    owner                   TEXT        NOT NULL,
    strategyManager         TEXT        NOT NULL,

    deviation_increase_fee  INT         NOT NULL,
    deviation_limit         INT         NOT NULL,
    cashback_fee            INT         NOT NULL,
    base_fee                INT         NOT NULL,
    management_fee_receiver TEXT        NOT NULL,
    management_fee          INT         NOT NULL,
    total_target_shares     INT         NOT NULL,
    initial_share_price     NUMERIC     NOT NULL,
    oracle_address          TEXT        NOT NULL,

    volumes_24h             NUMERIC     NOT NULL,    
    CONSTRAINT multipools_pkey  PRIMARY KEY (chain_id, multipool_id)
);

CREATE TABLE IF NOT EXISTS events 
(
    event_id            BIGSERIAL   NOT NULL,
    chain_id            NUMERIC     NOT NULL,
    emitter_address     TEXT        NOT NULL,
    block_number        NUMERIC     NOT NULL,
    block_timestamp     BIGINT          NULL,
    transaction_hash    TEXT        NOT NULL,
    event_index         BIGINT      NOT NULL,
    event               JSON        NOT NULL,
    row_event           JSON        NOT NULL
);

CREATE TABLE IF NOT EXISTS trades 
(
    chain_id            BIGSERIAL   NOT NULL,
    multipool_address   TEXT        NOT NULL,
    block_number        NUMERIC  NOT NULL,
    block_timestamp     NUMERIC  NOT NULL,
    transaction_hash    TEXT        NOT NULL,
    event_index         BIGINT      NOT NULL,

    asset_in_address    TEXT        NOT NULL,
    asset_out_address   TEXT        NOT NULL,
    asset_in_amount     NUMERIC     NOT NULL,
    asset_out_amount    NUMERIC     NOT NULL,
    asset_in_price      NUMERIC     NOT NULL,
    asset_out_price     NUMERIC     NOT NULL,
    sender_address      NUMERIC     NOT NULL
);

CREATE TABLE IF NOT EXISTS multipool_assets
(
    asset_address   TEXT    NOT NULL,
    quantity        NUMERIC NOT NULL,
    cashbacks       NUMERIC NOT NULL,
    price_data      TEXT    NOT NULL,
    target_share    INT     NOT NULL,
    chain_id        INT     NOT NULL,
    price           NUMERIC NOT NULL,

    name            TEXT        NULL,
    symbol          TEXT        NULL,
    logo_url        TEXT        NULL,
    description     TEXT        NULL,

    PRIMARY KEY (chain_id, asset_address)
);

CREATE TABLE IF NOT EXISTS candles
(
    multipool_id        INT     NOT NULL,
    ts                  BIGINT  NOT NULL,
    resolution          INT     NOT NULL,
    open                NUMERIC NOT NULL,
    close               NUMERIC NOT NULL,
    low                 NUMERIC NOT NULL,
    high                NUMERIC NOT NULL,
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

    --                         1m 15m 30m  60m   12h   24h    
    var_resolutions INT[] := '{60,900,1800,3600,43200,86400}';
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
