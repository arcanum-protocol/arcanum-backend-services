use ethers::types::Address;

#[derive(Debug, Clone)]
pub enum MultipoolErrors {
    Overflow(MultipoolOverflowErrors),
    QuotedQuantityMissing(Address),
    QuantitySlotMissing(Address),
    AssetMissing(Address),
    PriceMissing(Address),
    TotalSupplyMissing(Address),
    ShareMissing(Address),
    TotalSharesMissing(Address),
}

#[derive(Debug, Clone)]
pub enum MultipoolOverflowErrors {
    QuotedQuantityOverflow,
    TargetShareOverflow,
    TargetDeviationOverflow,
    PriceCapOverflow,
    TotalSupplyOverflow,
    CurrentShareTooBig,
    TargetShareTooBig,
}
