CREATE TABLE IF NOT EXISTS trader_stats
(
    asset_in_address TEXT NOT NULL,
    asset_out_address TEXT NOT NULL,
    strategy TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    multipool_id TEXT NOT NULL,

    row_timestamp BIGINT NOT NULL,

    pool_in_address TEXT NOT NULL,
    pool_out_address TEXT NOT NULL,

    profit_ratio numeric NOT NULL,
    strategy_input numeric NOT NULL,
    strategy_output numeric NOT NULL,
    multipool_fee numeric NOT NULL,
    multipool_amount_in numeric NOT NULL,
    multipool_amount_out numeric NOT NULL,

    asset_in_symbol TEXT NOT NULL,
    asset_out_symbol TEXT NOT NULL,

    multipool_asset_in_price numeric NOT NULL,
    multipool_asset_out_price numeric NOT NULL,

    pool_in_fee INT NOT NULL,
    pool_out_fee INT NOT NULL,

    estimation_error TEXT NULL,
    estimated_gas numeric NULL,
    estimated_profit numeric NULL,
    PRIMARY KEY (multipool_id, asset_in_address, asset_out_address, strategy, timestamp)
);
