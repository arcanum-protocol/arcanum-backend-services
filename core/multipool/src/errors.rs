use ethers::types::Address;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum MultipoolOverflowErrors {
    QuotedQuantityOverflow,
    TargetDeviationOverflow,
    PriceCapOverflow,
    TotalSupplyOverflow,
}
