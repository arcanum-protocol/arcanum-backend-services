use alloy::primitives::Address;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultipoolErrors {
    Overflow(MultipoolOverflowErrors),
    QuotedQuantityMissing(Address),
    ZeroCap,
    AssetMissing(Address),
    PriceMissing(Address),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultipoolOverflowErrors {
    QuotedQuantityOverflow,
    TargetDeviationOverflow,
    TotalSharesOverflow(Address),
    PriceCapOverflow,
    TotalSupplyOverflow,
}
