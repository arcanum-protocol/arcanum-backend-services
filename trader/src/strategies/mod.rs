use alloy::{
    dyn_abi::DynSolValue,
    primitives::{Address, Bytes, I256, U256},
    providers::Provider,
};
use anyhow::{anyhow, bail, Result};
use multipool_types::{expiry::StdTimeExtractor, Multipool::OraclePrice};
use std::ops::Shr;

use crate::{
    contracts::{multipool::MultipoolContract, SiloLens, ERC20, SILO_LENS, SILO_WRAPPER},
    trade::{AssetsChoise, MultipoolChoise, WrapperCall},
};

impl<P: Provider + Clone> AssetsChoise<P> {
    pub async fn estimate_multipool(self) -> Result<MultipoolChoise<P>> {
        let price_in = self
            .trading_data
            .multipool
            .get_price(&self.asset1)?;
        let price_out = self
            .trading_data
            .multipool
            .get_price(&self.asset2)?;
        let amount = self
        .trading_data
        .multipool
            .quantity_to_deviation(&self.asset1, self.deviation_bound)
            .map_err(|v| anyhow!("{v:?}"))?;


        let amount_out = U256::from(amount.abs()) * price_in / price_out;

        if amount.is_negative() {
            bail!(anyhow!("Amount in negative"));
        }

        let mp = MultipoolContract::new(self
            .trading_data
            .multipool.address, self
            .trading_data
            .rpc.clone());

        let res = mp.estimate_swap(
            MultipoolContract::OraclePrice {
                contractAddress: Address::ZERO,
                timestamp: 0,
                sharePrice: 0,
                signature: Bytes::new()
            },
            self.asset1, 
            self.asset2,
            U256::from(amount.abs()), 
            true).call().await?;

        println!("ESTIMATE  {} {} {} {} and calculated {}", res.amountIn, res.amountOut, res.fees, res.cashbacks, amount_out);

        let fee: I256 = I256::unchecked_from(res.fees);

        let amount_of_in = I256::from_raw(res.amountIn);
        // let amount_of_out = I256::unchecked_from(-1000000);
        let amount_of_out = I256::unchecked_from(res.amountOut);

        let multipool_amount_in = U256::try_from(amount_of_in.abs())?;
        let multipool_amount_out = U256::try_from(amount_of_out.abs())?;

        let (unwrapped_amount_in, swap_asset_in, wrap_call) = if let Some((silo_pool, base_asset)) =
            self.trading_data.silo_assets.get(&self.asset1)
        {
            let total_supply = ERC20::new(self.asset1, self.trading_data.rpc.clone())
                .totalSupply()
                .call()
                .await?;
            let collected = SiloLens::new(SILO_LENS, self.trading_data.rpc.clone())
                .totalDepositsWithInterest(*silo_pool, *base_asset)
                .call()
                .await?;

            (
                multipool_amount_in * collected / total_supply,
                *base_asset,
                WrapperCall {
                    wrapper: SILO_WRAPPER,
                    data: DynSolValue::Tuple(vec![
                        DynSolValue::Address(*silo_pool),
                        DynSolValue::Address(*base_asset),
                        DynSolValue::Address(self.asset1),
                    ])
                    .abi_encode(),
                },
            )
        } else {
            (
                multipool_amount_in,
                self.asset1,
                WrapperCall {
                    wrapper: Address::ZERO,
                    data: Vec::default(),
                },
            )
        };

        let (unwrapped_amount_out, swap_asset_out, unwrap_call) = if let Some((
            silo_pool,
            base_asset,
        )) =
            self.trading_data.silo_assets.get(&self.asset2)
        {
            let total_supply = ERC20::new(self.asset2, self.trading_data.rpc.clone())
                .totalSupply()
                .call()
                .await?;

            let collected = SiloLens::new(SILO_LENS, self.trading_data.rpc.clone())
                .totalDepositsWithInterest(*silo_pool, *base_asset)
                .call()
                .await?;

            (
                multipool_amount_out * collected / total_supply,
                *base_asset,
                WrapperCall {
                    wrapper: SILO_WRAPPER,
                    data: DynSolValue::Tuple(vec![
                        DynSolValue::Address(*silo_pool),
                        DynSolValue::Address(*base_asset),
                        DynSolValue::Address(self.asset2),
                    ])
                    .abi_encode(),
                },
            )
        } else {
            (
                multipool_amount_out,
                self.asset2,
                WrapperCall {
                    wrapper: Address::ZERO,
                    data: Vec::default(),
                },
            )
        };

        Ok(MultipoolChoise {
            trading_data_with_assets: self,

            swap_asset_in,
            swap_asset_out,

            multipool_amount_in,
            multipool_amount_out,

            unwrapped_amount_in,
            unwrapped_amount_out,

            wrap_call,
            unwrap_call,

            fee,
        })
    }
}
//  assets: {
//     0xb2f82d0f38dc453d596ad40a37799446cc89274a: MpAsset { 
//         address: 0xb2f82d0f38dc453d596ad40a37799446cc89274a, 
//         quantity: 3769742731863919824, 
//         price: 79099144920519128186171538256, 
//         target_share: 10 
//     }, 
//     0x0f0bdebf0f83cd1ee3974779bcb7315f9808c714: MpAsset {
//         address: 0x0f0bdebf0f83cd1ee3974779bcb7315f9808c714, 
//         quantity: 10696324732792741825, 
//         price: 272335392985689149155207077056, 
//         target_share: 10 
//     }, 
//     0xe0590015a873bf326bd645c3e1266d4db41c4e6b: MpAsset { 
//         address: 0xe0590015a873bf326bd645c3e1266d4db41c4e6b, 
//         quantity: 9175102860042797953, 
//         price: 1563403557198007219922291894, 
//         target_share: 10 
//     }, 
//     0xfe140e1dce99be9f4f15d657cd9b7bf622270c50: MpAsset { 
//         address: 0xfe140e1dce99be9f4f15d657cd9b7bf622270c50, 
//         quantity: 22596109291089760153, 
//         price: 145101055232530068641984591, 
//         target_share: 10 
//     }, 
//     0xaeef2f6b429cb59c9b2d7bb2141ada993e8571c3: MpAsset { 
//         address: 0xaeef2f6b429cb59c9b2d7bb2141ada993e8571c3, 
//         quantity: 845558493087915468, 
//         price: 79604018667121319057066105674, 
//         target_share: 10 
//     }}, context: MpContext { sharePrice: 79228162514264337593543950336, oldTotalSupply: 16967939447406380801952938, totalSupplyDelta: 0, totalTargetShares: 50, deviationIncreaseFee: 0, deviationLimit: 4294967296, feeToCashbackRatio: 0, baseFee: 0, managementBaseFee: 0, deviationFees: 0, collectedCashbacks: 0, collectedFees: 0, managementFeeRecepient: 0x65fc395ec32d69551b3966f8e5323fd233a8c9ec, oracleAddress: 0x97cd13624bb12d4ec39469b140f529459d5d369d }, cap: 194254829721914606011583 }