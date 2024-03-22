use ethers::types::Address;

#[derive(Debug, Clone, PartialEq)]
pub enum MultipoolErrors {
    Overflow(MultipoolOverflowErrors),
    QuotedQuantityMissing(Address),
    QuantitySlotMissing(Address),
    QuantitySlotQuantitySlotMissing(Address),
    AssetMissing(Address),
    PriceMissing(Address),
    TotalSupplyMissing(Address),
    ShareMissing(Address),
    TotalSharesMissing(Address),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MultipoolOverflowErrors {
    QuotedQuantityOverflow,
    TargetDeviationOverflow,
    PriceCapOverflow,
    TotalSupplyOverflow,
}
