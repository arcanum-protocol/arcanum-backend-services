CREATE DOMAIN ADDRESS AS BYTEA
CHECK (LENGTH(VALUE) = 20);

CREATE DOMAIN BYTES32 AS BYTEA
CHECK (LENGTH(VALUE) = 32);

CREATE DOMAIN I256 AS numeric(79,0);

CREATE DOMAIN U256 AS numeric(78,0)
CONSTRAINT u256_check CHECK (VALUE >= 0);

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

    quantity            U256    NOT NULL,

    profit              U256 NOT NULL,
    loss                U256 NOT NULL,

    opened_at           BIGINT  NOT NULL,

    CONSTRAINT positions_pkey PRIMARY KEY (chain_id, account, multipool)
);

CREATE TABLE IF NOT EXISTS positions_history
(
    chain_id            BIGINT  NOT NULL,
    account             ADDRESS NOT NULL,
    multipool           ADDRESS NOT NULL,

    pnl_percent         U256    NOT NULL,
    pnl_quantity        U256    NOT NULL,

    opened_at           BIGINT  NOT NULL,
    closed_at           BIGINT  NOT NULL,

    UNIQUE (chain_id, account, multipool, opened_at)
);

CREATE TABLE IF NOT EXISTS actions_history
(
    chain_id            BIGINT          NOT NULL,
    account             ADDRESS         NOT NULL,
    multipool           ADDRESS         NOT NULL,

    quantity            I256 NOT NULL,
    quote_quantity      I256 NOT NULL,

    transaction_hash    BYTES32 NOT NULL,
    block_number        BIGINT  NOT NULL,
    timestamp           BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS multipools
(
    name                TEXT        NULL,
    symbol              TEXT        NULL,
    description         TEXT        NULL,
    logo                BYTEA       NULL,

    chain_id            BIGINT  NOT NULL,
    multipool           ADDRESS NOT NULL,
    owner               ADDRESS NOT NULL,

    total_supply        U256    NOT NULL DEFAULT '0'
);

CREATE TABLE IF NOT EXISTS candles
(
    multipool           ADDRESS NOT NULL,
    resolution          INT     NOT NULL,
    ts                  BIGINT  NOT NULL,

    open                U256 NOT NULL,
    close               U256 NOT NULL,
    low                 U256 NOT NULL,
    hight               U256 NOT NULL,

    CONSTRAINT candles_pkey PRIMARY KEY (multipool, resolution, ts)
);

-- price is decimal with precision 10^6
CREATE OR REPLACE PROCEDURE insert_price(arg_multipool ADDRESS, arg_timestamp BIGINT, arg_new_price U256)
LANGUAGE plpgsql
AS $$
DECLARE
    --                         1m 15m 60m  24h
    var_resolutions INT[] := '{60,900,3600,86400}';
    var_resol INT;
BEGIN

        IF (select multipool from multipools where multipool=arg_multipool limit 1) IS NULL THEN
            insert into multipools(multipool) values (arg_multipool);
        END IF;

        -- gen candles
        FOREACH var_resol in array var_resolutions
        LOOP
            INSERT INTO candles(multipool, ts, resolution, open, close, low, hight)
            VALUES(
                arg_multipool,
                arg_timestamp / var_resol * var_resol,
                var_resol,
                arg_new_price,
                arg_new_price,
                arg_new_price,
                arg_new_price
                )
            ON CONFLICT (multipool, resolution, ts) DO UPDATE SET
                close = arg_new_price,
                low = least(candles.low, arg_new_price),
                hight = greatest(candles.hight, arg_new_price);
        END LOOP;
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
