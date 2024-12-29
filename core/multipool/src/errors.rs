use alloy::primitives::Address;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultipoolErrors {
    Overflow(MultipoolOverflowErrors),
    QuotedQuantityMissing(Address),
    ZeroCap,
    QuantitySlotMissing(Address),
    QuantitySlotQuantitySlotMissing(Address),
    AssetMissing(Address),
    PriceMissing(Address),
    TotalSupplyMissing(Address),
    ShareMissing(Address),
    TotalSharesMissing(Address),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultipoolOverflowErrors {
    QuotedQuantityOverflow,
    TargetDeviationOverflow,
    TotalSharesOverflow(Address),
    PriceCapOverflow,
    TotalSupplyOverflow,
}
