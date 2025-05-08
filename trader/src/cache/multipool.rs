use alloy::providers::MulticallBuilder;
use alloy::{
    primitives::{Address, Bytes, I256, U256, U512},
    providers::{Failure, Provider},
};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::ops::Shl;
use std::ops::Shr;
use std::ops::Mul;

use crate::contracts::multipool::MultipoolContract::{
    self, MpAsset as ContractMpAsset, MpContext, OraclePrice,
};

#[derive(Clone, Debug)]
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

        let assets: Vec<ContractMpAsset> = MulticallBuilder::new_dynamic(rpc)
            .extend(get_asset_calls)
            .aggregate3_value()
            .await?
            .into_iter()
            .collect::<Result<Vec<ContractMpAsset>, Failure>>().map_err(|e| anyhow!("get assets {e}"))?;
        let prices: Vec<U256> = MulticallBuilder::new_dynamic(rpc)
            .extend(get_price_calls)
            .aggregate3_value()
            .await?
            .into_iter()
            // .map(|v| v.map(|s| s.shr(96)))
            // .map(|v| v.map(|s| s / U256::from(2^96)))
            .collect::<Result<Vec<U256>, Failure>>().map_err(|e| anyhow!("get prices {e}"))?;

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

    pub fn quantity_to_deviation_new(
        &self,
        asset_address: &Address,
        target_deviation: I256,
    ) -> Result<I256> {
        let asset = self.assets.get(asset_address).context("No asset")?;
        let quantity = asset.quantity;
        let price = asset.price;
        let share = self.current_share(asset)?;
        let total_target_shares = self.context.totalTargetShares;
        // 6,938,514.696586093916610000
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

    pub fn quantity_to_deviation(
        &self,
        asset_address: &Address,
        target_deviation: I256,
    ) -> Result<I256> {
        let asset = self.assets.get(asset_address).context("No asset")?;
        // let quantity = asset.quantity;
        let price = asset.price;
        let cap_share = asset.quantity * asset.price;
        // let cap_share = self.current_share(asset)?;
        let total_target_shares = self.context.totalTargetShares;
        // 6,938,514.696586093916610000
        let usd_cap = self.cap()?;
        // share / total_shares = cap_share / cap
        let must_be_share = asset.target_share * usd_cap / total_target_shares;
        let deviated_share = I256::from(must_be_share) - I256::from(must_be_share) * target_deviation;
        let amount = (deviated_share - I256::from(cap_share)) / I256::from(price); 
        Ok(amount)
    }

    pub fn optimal_amount_in(
        &self,
        token_in: &Address,
        token_out: &Address,
    ) -> Result<U256> {
        let asset_in = self.assets.get(token_in).context("No asset")?;
        let asset_out = self.assets.get(token_out).context("No asset")?;

        let total_supply = self.context.oldTotalSupply;
        let current_share_in = (asset_in.quantity * asset_in.price) / total_supply;
        let target_share_in = (asset_in.target_share << 32) / self.context.totalTargetShares;

        let deviation = if current_share_in > target_share_in {
            current_share_in - target_share_in 
        } else {
            target_share_in - current_share_in  
        };
        let full_rebalance_amount = (deviation * total_supply) / asset_in.price;
        Ok((full_rebalance_amount * U256::from(95)) / U256::from(100))
    }
}


// Multipool { 
//     address: 0x057a931a8ab1111ff163745de18040dc0b35f153, 
//     assets_addresses: [0xb2f82d0f38dc453d596ad40a37799446cc89274a, 0x0f0bdebf0f83cd1ee3974779bcb7315f9808c714, 0xe0590015a873bf326bd645c3e1266d4db41c4e6b, 0xfe140e1dce99be9f4f15d657cd9b7bf622270c50, 0xaeef2f6b429cb59c9b2d7bb2141ada993e8571c3], 
//     assets: {
//         0xaeef2f6b429cb59c9b2d7bb2141ada993e8571c3: MpAsset { 
//             address: 0xaeef2f6b429cb59c9b2d7bb2141ada993e8571c3, 
//             quantity: 49861994848346826, 
//             price: 10000, 
//             target_share: 10 
//         }, 
//         0xb2f82d0f38dc453d596ad40a37799446cc89274a: MpAsset { 
//             address: 0xb2f82d0f38dc453d596ad40a37799446cc89274a, 
//             quantity: 100394231864016244, 
//             price: 100000, 
//             target_share: 10 
//         }, 
//         0xe0590015a873bf326bd645c3e1266d4db41c4e6b: MpAsset { 
//             address: 0xe0590015a873bf326bd645c3e1266d4db41c4e6b, 
//             quantity: 4088065494294338907, 
//             price: 300000, 
//             target_share: 10 
//         }, 
//         0xfe140e1dce99be9f4f15d657cd9b7bf622270c50: MpAsset { 
//             address: 0xfe140e1dce99be9f4f15d657cd9b7bf622270c50, 
//             quantity: 22596109291089760153, 
//             price: 250000, 
//             target_share: 10 
//         }, 
//         0x0f0bdebf0f83cd1ee3974779bcb7315f9808c714: MpAsset { 
//             address: 0x0f0bdebf0f83cd1ee3974779bcb7315f9808c714, 
//             quantity: 262648411952335568, 
//             price: 200000, 
//             target_share: 10 
//         }
//     }, 
//     context: MpContext { 
//         sharePrice: 79228162514264337593543950336, 
//         oldTotalSupply: 16967899151697031647617299, 
//         totalSupplyDelta: 0, 
//         totalTargetShares: 50, 
//         deviationIncreaseFee: 0, 
//         deviationLimit: 4294967296, 
//         feeToCashbackRatio: 0, 
//         baseFee: 0, 
//         managementBaseFee: 0, 
//         deviationFees: 0, 
//         collectedCashbacks: 0, 
//         collectedFees: 0, 
//         managementFeeRecepient: 0x65fc395ec32d69551b3966f8e5323fd233a8c9ec, 
//         oracleAddress: 0x97cd13624bb12d4ec39469b140f529459d5d369d 
//     }, 
//     cap: 32397986637830420594633729233 
// }
