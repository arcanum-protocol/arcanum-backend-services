import { EventData } from "web3-eth-contract";
import { BlockNumber } from "./types";
import { PoolClient } from "pg";

export enum EventType {
    AssetPercentsChange = "AssetPercentsChange",
    AssetQuantityChange = "AssetQuantityChange",
    AssetPriceChange = "AssetPriceChange",
}

export interface Log {
    data: {[key: string]: any},
    block_number: BlockNumber,
    event_type: string,
    process(db: PoolClient): Promise<undefined>;
};


export class DataLog implements Log {
    constructor (event: EventData) {
        this.data = event.returnValues;
        this.block_number = event.blockNumber;
        this.event_type = EventType[event.event as keyof typeof EventType];
    }

    data: {[key: string]: any};
    block_number: BlockNumber;
    event_type: EventType;

    async process(db: PoolClient): Promise<undefined> {
        const method = event_mapping[this.event_type];
        method(db)
    }

}

const event_mapping: {[key in EventType]: (db: PoolClient) => Promise<undefined>} = {
    // 
    "AssetPercentsChange": async (db: PoolClient): Promise<undefined> => {

    },
    // 
    "AssetQuantityChange": async (db: PoolClient): Promise<undefined> => {

    },
    // 
    "AssetPriceChange": async (db: PoolClient): Promise<undefined> => {

    },
}
