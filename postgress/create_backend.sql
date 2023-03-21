CREATE TABLE IF NOT EXISTS assets
(
    name TEXT NOT NULL UNIQUE,
    address TEXT NOT NULL UNIQUE,
    coingecko_id TEXT NOT NULL UNIQUE,
    price numeric NOT NULL,
    mcap numeric NOT NULL
);

CREATE TABLE IF NOT EXISTS prices
(
    ts bigint NOT NULL,
    price numeric NOT NULL,
    CONSTRAINT price_pkey PRIMARY KEY (ts)
);

CREATE OR REPLACE FUNCTION update_price_function() RETURNS trigger AS $uc$
DECLARE
    total_mcap BIGINT;
    new_price numeric;
BEGIN

    NEW.price = ROUND(NEW.price, 6);
    NEW.mcap = ROUND(NEW.mcap, 6);

    SELECT SUM(mcap) INTO total_mcap FROM assets;

    SELECT SUM(price * mcap) / total_mcap INTO new_price FROM assets;


    INSERT INTO prices(ts, price)
    VALUES((EXTRACT(epoch FROM CURRENT_TIMESTAMP(0)::TIMESTAMP WITHOUT TIME ZONE))::BIGINT / 3600 * 3600, new_price)
    ON CONFLICT (ts) DO UPDATE SET
        price = ROUND((prices.price + new_price) / 2, 6);
	RETURN NEW;
END; $uc$ LANGUAGE plpgsql;

CREATE TRIGGER update_price
    AFTER UPDATE
    ON assets
    FOR EACH STATEMENT
    EXECUTE FUNCTION update_price_function();
