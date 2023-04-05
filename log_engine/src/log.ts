import { EventData } from "web3-eth-contract";
import { BlockNumber } from "./types";
import { PoolClient } from "pg";

export enum EventType {
    AssetPercentsChange = "AssetPercentsChange",
    AssetQuantityChange = "AssetQuantityChange",
    AssetPriceChange = "AssetPriceChange",
}

export const assemble_log: { [key in EventType]: (db: PoolClient, data: { [key: string]: any }, address: string) => Promise<void> } = {
    // 
    "AssetPercentsChange": async (db: PoolClient, event: { [key: string]: any }, address: string): Promise<void> => {
        db.query("UPDATE multipool SET percent = $1 WHERE address = $2", [event.percent, address]);
    },
    // 
    "AssetQuantityChange": async (db: PoolClient, event: { [key: string]: any }, address: string): Promise<void> => {
        db.query("UPDATE multipool SET quantity = $1 WHERE address = $2", [event.quantity, address]);
    },
    // 
    "AssetPriceChange": async (db: PoolClient, event: { [key: string]: any }, address: string): Promise<void> => {
        db.query("UPDATE multipool SET price = $1 WHERE address = $2", [event.price, address]);
    },
}
