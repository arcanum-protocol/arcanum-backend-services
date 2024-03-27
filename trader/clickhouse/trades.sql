CREATE OR REPLACE TABLE trades
(
    detection_timestamp     DateTime64(6, 'UTC')    NOT NULL DEFAULT NOW(),

    multipool_address       FixedString(42)         NOT NULL ,

    trade_input             UInt256                 NOT NULL,
    trade_output            UInt256                 NOT NULL,
    multipool_fee           Int256                  NOT NULL,

    asset_in_address        FixedString(42)         NOT NULL,
    asset_out_address       FixedString(42)         NOT NULL,

    pool_in_address         FixedString(42)         NOT NULL,
    pool_in_fee             UInt32                  NOT NULL,

    pool_out_address        FixedString(42)         NOT NULL,
    pool_out_fee            UInt32                  NOT NULL,

    multipool_amount_in     UInt256                 NOT NULL,
    multipool_amount_out    UInt256                 NOT NULL,

    strategy_type           LowCardinality(String)  NOT NULL,

    is_profitable           BOOL                    NOT NULL MATERIALIZED profit > 0,

    profit                  Int256                  NOT NULL MATERIALIZED toInt256(trade_output) - toInt256(trade_input),
    profit_with_fees        Int256                  NOT NULL MATERIALIZED toInt256(trade_output) - toInt256(trade_input) - multipool_fee,

    profix_ratio            Decimal256(18)          NOT NULL MATERIALIZED toDecimal256(trade_output, 18) / toDecimal256(trade_input, 18),
    profix_ratio_with_fees  Decimal256(18)          NOT NULL MATERIALIZED (toDecimal256(trade_output, 18) - toDecimal256(multipool_fee, 18)) / toDecimal256(trade_input, 18)
)
ENGINE = MergeTree
ORDER BY detection_timestamp;
