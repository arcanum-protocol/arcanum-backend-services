CREATE TABLE IF NOT EXISTS trader_stats
(
    asset_in_address TEXT NOT NULL,
    asset_out_address TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    PRIMARY KEY (asset_in_address, asset_out_address, timestamp),

    row_timestamp BIGINT NOT NULL,

    pool_in_address TEXT NOT NULL,
    pool_out_address TEXT NOT NULL,
    strategy TEXT NOT NULL,

    profit_ratio numeric NOT NULL,
    strategy_input numeric NOT NULL,
    strategy_output numeric NOT NULL,
    multipool_fee numeric NOT NULL,
    multipool_amount_in numeric NOT NULL,
    multipool_amount_out numeric NOT NULL,

    estimation_error TEXT NULL,
    estimated_gas numeric NULL,
    estimated_profit numeric NULL
);
