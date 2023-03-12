-- Table: public.assets

-- DROP TABLE IF EXISTS public.assets;

CREATE TABLE IF NOT EXISTS public.assets
(
    name text COLLATE pg_catalog."default" NOT NULL,
    address text COLLATE pg_catalog."default" NOT NULL,
    price numeric NOT NULL,
    mcap bigint NOT NULL
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.assets
    OWNER to postgres;

-- Trigger: update_candles

-- DROP TRIGGER IF EXISTS update_candles ON public.assets;

CREATE TRIGGER update_candles
    AFTER UPDATE 
    ON public.assets
    FOR EACH STATEMENT
    EXECUTE FUNCTION public.update_candles();

-- Table: public.candles

-- DROP TABLE IF EXISTS public.candles;

CREATE TABLE IF NOT EXISTS public.candles
(
    ts bigint NOT NULL,
    open numeric NOT NULL,
    close numeric NOT NULL,
    high numeric,
    low numeric,
    CONSTRAINT candles_pkey PRIMARY KEY (ts)
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.candles
    OWNER to postgres;

DECLARE
    total_mcap BIGINT;
    index_price DECIMAL;
BEGIN
    -- Calculate total market capitalization
    SELECT SUM(mcap) INTO total_mcap FROM assets;

    -- Calculate index price
    SELECT SUM(price * mcap) / total_mcap INTO index_price FROM assets;

    -- Insert or update candle
    INSERT INTO candles(ts, open, close, high, low)
    VALUES((EXTRACT(epoch FROM now()) * 60) / 60, index_price, index_price, index_price, index_price)
    ON CONFLICT (ts) DO UPDATE SET
        close = excluded.close,
        high = GREATEST(candles.high, excluded.close),
        low = LEAST(candles.low, excluded.close);
	RETURN NEW;
END;