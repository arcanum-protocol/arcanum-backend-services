import BigNumber from 'bignumber.js';
import { PoolClient } from 'pg';
import { AbiItem } from 'web3-utils'

export type Address = string;
export type BlockNumber = number;
export interface Percents {
    address: Address,
    percent: BigNumber
}

export interface Quantity {
    address: Address,
    quantity: BigNumber
}

export interface Price {
    address: Address,
    price: BigNumber
}

export interface Config {
    from_block: BlockNumber,
    event: string | null,
};

export interface ContractConfig {
    address: Address,
    provider_url: string,
    abi: AbiItem[],
};
