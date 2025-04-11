create table if not exists blocks (
    chain_id            INT         NOT NULL, 
    block_number        NUMERIC     NOT NULL, 
    raw_block           JSONB       NOT NULL
);