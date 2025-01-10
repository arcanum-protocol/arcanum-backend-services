-- Add migration script here
CREATE TABLE raw_events (
    id SERIAL PRIMARY KEY,
    contract_address TEXT NOT NULL,
    chain_id TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    event_data JSONB NOT NULL,
    block_timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
);

CREATE TABLE chains (
    id SERIAL PRIMARY KEY,
    chain_id TEXT NOT NULL,
    last_observed_block BIGINT NOT NULL,
)