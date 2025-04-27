--CREATE TABLE IF NOT EXISTS chains (
--    chain_id BIGSERIAL PRIMARY KEY
--);

CREATE DOMAIN ADDRESS AS BYTEA
CHECK (LENGTH(VALUE) = 20);

CREATE DOMAIN BYTES32 AS BYTEA
CHECK (LENGTH(VALUE) = 32);

create table if not exists blocks (
    chain_id            BIGINT  NOT NULL, 
    block_number        BIGINT  NOT NULL, 
    payload             JSONB       NOT NULL,

    CONSTRAINT blocks_pkey PRIMARY KEY (chain_id, block_number)
);

create table if not exists price_indexes (
    chain_id        BIGINT  PRIMARY KEY, 
    block_number    BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS positions
(
    chain_id            BIGINT      NOT NULL,
    account             ADDRESS     NOT NULL,
    multipool           ADDRESS     NOT NULL,

    quantity            NUMERIC NOT NULL,

    profit              NUMERIC NOT NULL,
    loss                NUMERIC NOT NULL,

    opened_at           BIGINT  NOT NULL,

    CONSTRAINT positions_pkey PRIMARY KEY (chain_id, account, multipool)
);

CREATE TABLE IF NOT EXISTS positions_history
(
    chain_id            BIGINT  NOT NULL,
    account             ADDRESS NOT NULL,
    multipool           ADDRESS NOT NULL,

    pnl_percent         NUMERIC NOT NULL,
    pnl_quantity        NUMERIC NOT NULL,

    opened_at           BIGINT  NOT NULL,
    closed_at           BIGINT  NOT NULL,

    UNIQUE (chain_id, account, multipool, opened_at)
);

CREATE TABLE IF NOT EXISTS actions_history
(
    chain_id            BIGINT          NOT NULL,
    account             ADDRESS         NOT NULL,
    multipool           ADDRESS         NOT NULL,

    quantity            NUMERIC NOT NULL,
    quote_quantity      NUMERIC NOT NULL,

    transaction_hash    BYTES32 NOT NULL,
    block_number        BIGINT  NOT NULL,
    timestamp           BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS multipools
(
    name                TEXT        NULL,
    symbol              TEXT        NULL,

    chain_id            BIGINT  NOT NULL,
    multipool           ADDRESS NOT NULL,
    owner               ADDRESS NOT NULL,

    change_24h          NUMERIC     NULL,
    low_24h             NUMERIC     NULL,
    hight_24h           NUMERIC     NULL,
    current_price       NUMERIC     NULL,
    total_supply        NUMERIC NOT NULL DEFAULT '0'
);

CREATE TABLE IF NOT EXISTS candles
(
    multipool           ADDRESS NOT NULL,
    resolution          INT     NOT NULL,
    ts                  BIGINT  NOT NULL,

    open                NUMERIC NOT NULL,
    close               NUMERIC NOT NULL,
    low                 NUMERIC NOT NULL,
    hight               NUMERIC NOT NULL,

    CONSTRAINT candles_pkey PRIMARY KEY (multipool, resolution, ts)
);

CREATE OR REPLACE PROCEDURE insert_price(arg_multipool ADDRESS, arg_timestamp BIGINT, arg_new_price numeric) 
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
            INSERT INTO candles(chain_id, multipool, ts, resolution, open, close, low, hight)
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
                hight = greatest(candles.hight, arg_new_price);
        END LOOP;
        
        SELECT 
            MAX(hight),
            MIN(low)
        INTO highest, lowest
        FROM 
            candles
        WHERE 
            chain_id=arg_chain_id and
            multipool=arg_multipool and 
            resolution=60 and
            ts > (arg_timestamp - 1440) / 60 * 60;

        SELECT open 
        INTO earliest 
        FROM candles  
        WHERE 
            chain_id=arg_chain_id and
            multipool=arg_multipool and 
            resolution=60 and
            ts > (arg_timestamp - 1440) / 60 * 60
        ORDER BY ts ASC
        LIMIT 1;

        UPDATE multipools 
        SET
            change_24h=CASE WHEN earliest <> 0 THEN ROUND((arg_new_price-earliest) * '100'::numeric /earliest,6) ELSE '0'::numeric END,
            low_24h=lowest,
            hight_24h=highest,
            current_price=arg_new_price
        WHERE
            multipool=arg_multipool AND chain_id=arg_chain_id;

END 
$$;

CREATE OR REPLACE FUNCTION update_positions()
RETURNS TRIGGER 
LANGUAGE plpgsql 
AS $$
DECLARE
    c_pos POSITIONS := (SELECT positions FROM positions WHERE account = NEW.account and chain_id = NEW.chain_id and multipool = NEW.multipool);
BEGIN
    IF c_pos IS NULL THEN
        INSERT INTO positions(chain_id, account, multipool, quantity, profit, loss, opened_at) 
        VALUES (NEW.chain_id, NEW.account, NEW.multipool, NEW.quantity, 0, NEW.quote_quantity, NEW.timestamp);
    ELSIF c_pos.quantity + NEW.quantity = 0 THEN
        INSERT INTO positions_history(chain_id, account, multipool, profit, loss, opened_at, closed_at) 
        VALUES (NEW.chain_id, NEW.account, NEW.multipool, c_pos.profit - NEW.quoted_quantity, c_pos.loss, c_pos.open_ts, NEW.timestamp);

        DELETE FROM positions 
        WHERE
                account     = NEW.account 
            and chain_id    = NEW.chain_id 
            and multipool   = NEW.multipool;
    ELSE
        UPDATE positions 
        SET 
            quantity = quantity + NEW.quantity,
            profit = profit + CASE WHEN NEW.quantity < 0 THEN -NEW.quote_quantity ELSE 0 END,
            loss = loss + CASE WHEN NEW.quantity > 0 THEN NEW.quote_quantity ELSE 0 END
        WHERE 
                account     = NEW.account 
            and chain_id    = NEW.chain_id 
            and multipool   = NEW.multipool;
    END IF;
    RETURN NULL; 
END 
$$;

CREATE TRIGGER trigger_trading_history
AFTER INSERT ON actions_history
FOR EACH ROW EXECUTE FUNCTION update_positions();

