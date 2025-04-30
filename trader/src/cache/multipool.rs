use alloy::providers::MulticallBuilder;
use alloy::{
    primitives::{Address, Bytes, I256, U256, U512},
    providers::{Failure, Provider},
};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::ops::Shl;
use std::ops::Shr;

use crate::contracts::multipool::MultipoolContract::{
    self, MpAsset as ContractMpAsset, MpContext, OraclePrice,
};

#[derive(Clone)]
pub struct Multipool {
    pub address: Address,
    pub assets_addresses: Vec<Address>,
    pub assets: HashMap<Address, MpAsset>,
    pub context: MpContext,
    pub cap: U256,
}

#[derive(Clone, Debug)]
pub struct MpAsset {
    pub address: Address,
    // pub share: U256,
    pub quantity: U256,
    pub price: U256,
    pub target_share: U256,
}

impl Multipool {
    pub async fn from_rpc<T: Provider + Sync + Send + 'static>(
        rpc: T,
        address: Address,
    ) -> Result<Self> {
        let mp = MultipoolContract::new(address, &rpc);
        let (context, assets, cap) = rpc
            .multicall()
            .add(mp.getContext(OraclePrice {
                contractAddress: Address::ZERO,
                timestamp: 0,
                sharePrice: 0,
                signature: Bytes::new(),
            }))
            .add(mp.getUsedAssets(U256::MAX, U256::MIN))
            .add(mp.getSharePricePart(U256::MAX, U256::MIN))
            .try_aggregate(false)
            .await.map_err(|e| anyhow!("get mp {e}"))?;
        let mut mp = Self {
            address,
            assets_addresses: assets?._0,
            context: context?,
            cap: cap?,
            assets: HashMap::new(),
        };
        mp.update_assets(&rpc).await?;
        Ok(mp)
    }

    pub async fn update_assets<T: Provider + Sync + Send + 'static>(
        &mut self,
        rpc: &T,
    ) -> Result<()> {
        let mp = MultipoolContract::new(self.address, &rpc);
        let get_asset_calls: Vec<_> = self
            .assets_addresses
            .iter()
            .map(|asset| mp.getAsset(*asset))
            .collect();
        let get_price_calls: Vec<_> = self
            .assets_addresses
            .iter()
            .map(|asset| mp.getPrice(*asset))
            .collect();
        println!("Updating addresses: {}", self.address);

        let assets: Vec<ContractMpAsset> = MulticallBuilder::new_dynamic(rpc)
            .extend(get_asset_calls)
            .aggregate3_value()
            .await?
            .into_iter()
            .collect::<Result<Vec<ContractMpAsset>, Failure>>().map_err(|e| anyhow!("get assets {e}"))?;
        println!("Updating prices: {}", self.address);
        let prices: Vec<U256> = MulticallBuilder::new_dynamic(rpc)
            .extend(get_price_calls)
            .aggregate3_value()
            .await?
            .into_iter()
            .map(|v| v.map(|s| s.shr(96)))
            .collect::<Result<Vec<U256>, Failure>>().map_err(|e| anyhow!("get prices {e}"))?;
        println!("prices ----- {:?}", prices);
        self.assets = assets
            .into_iter()
            .zip(self.assets_addresses.clone())
            .zip(prices)
            .map(|((a, address), price)| {
                (
                    address,
                    MpAsset {
                        address,
                        quantity: a.quantity,
                        target_share: a.targetShare,
                        price,
                    },
                )
            })
            .collect();
        Ok(())
    }

    pub fn cap(&self) -> Result<U256> {
        let merged_prices = self
            .assets
            .iter()
            .map(|(_, asset)| asset.price * asset.quantity)
            .collect::<Vec<_>>();
        Ok(merged_prices
            .iter()
            .fold(U256::ZERO, |a, b| a.checked_add(*b).unwrap()))
    }

    pub fn current_share(&self, asset: &MpAsset) -> Result<U256> {
        let quote = asset.price * asset.quantity;
        let cap = self.cap()?;
        quote
            .shl(32_u64)
            .checked_div(cap)
            .context(anyhow!("Current share overflow"))
    }

    pub fn get_price(&self, asset: &Address) -> Result<U256> {
        self.assets
            .get(asset)
            .map(|a| a.price)
            .context(anyhow!("No asset"))
    }

    pub fn quantity_to_deviation(
        &self,
        asset_address: &Address,
        target_deviation: I256,
    ) -> Result<I256> {
        let asset = self.assets.get(asset_address).context("No asset")?;
        let quantity = asset.quantity;
        let price = asset.price;
        let share = self.current_share(asset)?;
        let total_target_shares = self.context.totalTargetShares;

        let usd_cap = U512::from(self.cap()?);

        let share_bound = U256::try_from(target_deviation.checked_abs().unwrap())?
            .checked_mul(U256::from(total_target_shares))
            .unwrap()
            .checked_shr(32)
            .unwrap();

        let result_share = if target_deviation.ge(&I256::default()) {
            U512::from(
                U256::from(share)
                    .checked_add(share_bound)
                    .unwrap_or_default()
                    .min(U256::from(total_target_shares)),
            )
        } else {
            U512::from(
                U256::from(share)
                    .checked_sub(share_bound)
                    .unwrap_or_default(),
            )
        };

        let amount: I256 = I256::from_raw(
            (result_share
                .checked_mul(usd_cap.shl(96))
                .expect("multiply shouldn't overflow"))
            .checked_div(U512::from(price))
            .expect("price division shouldn't overflow")
            .checked_div(U512::from(total_target_shares))
            .expect("total shares division shouldn't overflow")
            .to(),
        );
        let amount = amount - I256::from_raw(U256::from(quantity));
        Ok(amount)
    }
}
