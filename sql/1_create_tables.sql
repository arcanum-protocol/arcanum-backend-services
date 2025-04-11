--CREATE TABLE IF NOT EXISTS chains (
--    chain_id BIGSERIAL PRIMARY KEY
--);

CREATE DOMAIN ADDRESS AS BYTEA
CHECK (LENGTH(VALUE) = 20);

CREATE DOMAIN BYTES32 AS BYTEA
CHECK (LENGTH(VALUE) = 32);

CREATE TABLE IF NOT EXISTS assets
(
   asset           ADDRESS NOT NULL,
   chain_id        BIGINT  NOT NULL,
   is_primary      BOOLEAN NOT NULL,

   name            TEXT        NULL,
   symbol          TEXT        NULL,
   logo_url        TEXT        NULL,
   description     TEXT        NULL,

   PRIMARY KEY (chain_id, asset)
);

CREATE TABLE IF NOT EXISTS positions
(
    chain_id            BIGINT  NOT NULL,
    account             ADDRESS NOT NULL,
    multipool           ADDRESS NOT NULL,

    quantity            NUMERIC NOT NULL,
    opened_at           BIGINT  NOT NULL,

    CONSTRAINT positions_pkey PRIMARY KEY (chain_id, account, multipool)
);

CREATE TABLE IF NOT EXISTS positions_pnl
(
    account             ADDRESS   NOT NULL,
    multipool           ADDRESS   NOT NULL,
    chain_id            BIGINT    NOT NULL,

    acc_profit          NUMERIC    NOT NULL,
    acc_loss            NUMERIC    NOT NULL,

    open_quantity       NUMERIC    NOT NULL,
    open_price          NUMERIC    NOT NULL,
    close_quantity      NUMERIC    NOT NULL,
    close_price         NUMERIC    NOT NULL,
    
    timestamp           BIGINT    NOT NULL,

    CONSTRAINT positions_pnl_pkey PRIMARY KEY (chain_id, account, multipool, timestamp)
);

CREATE TABLE IF NOT EXISTS pnl
(
    account             ADDRESS   NOT NULL,
    chain_id            BIGINT    NOT NULL,

    acc_profit          NUMERIC    NOT NULL,
    acc_loss            NUMERIC    NOT NULL,

    open_quote          NUMERIC    NOT NULL,
    close_quote         NUMERIC    NOT NULL,

    timestamp           BIGINT    NOT NULL,

    CONSTRAINT pnl_pkey PRIMARY KEY (chain_id, account, timestamp)
);

CREATE TYPE TRADING_ACTION AS ENUM ('mint', 'burn', 'send', 'receive');

CREATE TABLE IF NOT EXISTS trading_history
(
    account             ADDRESS         NOT NULL,
    multipool           ADDRESS         NOT NULL,
    chain_id            BIGINT          NOT NULL,

    action_type         TRADING_ACTION  NOT NULL,

    quantity            NUMERIC NOT NULL,
    quote_quantity      NUMERIC NOT NULL default 0,
    transaction_hash    BYTES32 NOT NULL,
    timestamp           BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS multipools
(
    name                TEXT        NULL,
    symbol              TEXT        NULL,
    description         TEXT        NULL,

    chain_id            BIGINT  NOT NULL,
    multipool           ADDRESS NOT NULL,
    change_24h          NUMERIC     NULL,
    low_24h             NUMERIC     NULL,
    high_24h            NUMERIC     NULL,
    current_price       NUMERIC     NULL,
    total_supply        NUMERIC NOT NULL DEFAULT '0'
);

CREATE TABLE IF NOT EXISTS candles
(
    chain_id            BIGINT  NOT NULL,
    multipool           ADDRESS NOT NULL,
    resolution          INT     NOT NULL,
    ts                  BIGINT  NOT NULL,

    open                NUMERIC NOT NULL,
    close               NUMERIC NOT NULL,
    low                 NUMERIC NOT NULL,
    high                NUMERIC NOT NULL,

    CONSTRAINT candles_pkey PRIMARY KEY (chain_id, multipool, resolution, ts)
);

CREATE OR REPLACE PROCEDURE insert_price(arg_chain_id BIGINT, arg_multipool ADDRESS, arg_timestamp BIGINT, arg_new_price numeric) 
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

        arg_new_price = ROUND(arg_new_price, 6);

        IF (select multipool from multipools where chain_id = arg_chain_id AND multipool=arg_multipool limit 1) IS NULL THEN
            insert into multipools(chain_id, multipool) values (arg_chain_id, arg_multipool);
        END IF;

        -- gen candles
        FOREACH var_resol in array var_resolutions
        LOOP 
            INSERT INTO candles(chain_id, multipool, ts, resolution, open, close, low, high)
            VALUES(
                arg_chain_id,
                arg_multipool,
                arg_timestamp / var_resol * var_resol, 
                var_resol,
                arg_new_price,
                arg_new_price,
                arg_new_price,
                arg_new_price
                )
            ON CONFLICT (chain_id, multipool, resolution, ts) DO UPDATE SET
                close = arg_new_price,
                low = least(candles.low, arg_new_price),
                high = greatest(candles.high, arg_new_price);
        END LOOP;
        
        SELECT 
            MAX(high),
            MIN(low)
        INTO highest, lowest
        FROM 
            candles
        WHERE 
            chain_id=arg_chain_id and
            multipool=arg_multipool and 
            resolution=60 and
            ts > (arg_timestamp - 86400) / 60 * 60;

        SELECT open 
        INTO earliest 
        FROM candles  
        WHERE 
            chain_id=arg_chain_id and
            multipool=arg_multipool and 
            resolution=60 and
            ts > (arg_timestamp - 86400) / 60 * 60
        ORDER BY ts ASC
        LIMIT 1;

        UPDATE multipools 
        SET
            change_24h=CASE WHEN earliest <> 0 THEN ROUND((arg_new_price-earliest) * '100'::numeric /earliest,6) ELSE '0'::numeric END,
            low_24h=lowest,
            high_24h=highest,
            current_price=arg_new_price
        WHERE
            multipool=arg_multipool AND chain_id=arg_chain_id;

        INSERT INTO positions_pnl(chain_id, account, multipool, timestamp, acc_profit, acc_loss, open_quantity, open_price, close_quantity, close_price)
            SELECT 
                chain_id, 
                account, 
                multipool, 
                arg_timestamp / 3600 * 3600, 
                acc_profit, 
                acc_loss, 
                p.close_quantity as open_quantity, 
                p.close_price as open_price,
                p.close_quantity as close_quantity, 
                arg_new_price as close_price
            FROM positions_pnl p WHERE 
                chain_id = arg_chain_id AND 
                multipool = arg_multipool AND 
                timestamp = arg_timestamp / 3600 * 3600 - 3600 AND
                close_quantity != 0
        ON CONFLICT (chain_id, account, multipool, timestamp) DO UPDATE SET
            close_price = arg_new_price;


END 
$$;
