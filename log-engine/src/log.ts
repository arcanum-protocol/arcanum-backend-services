import { EventData } from "web3-eth-contract";
import { BlockNumber } from "./types";
import { PoolClient } from "pg";

export enum EventType {
    AssetPercentsChange = "AssetPercentsChange",
    AssetQuantityChange = "AssetQuantityChange",
    AssetPriceChange = "AssetPriceChange",
}

export const assemble_log: { [key in EventType]: (db: PoolClient, data: { [key: string]: any }, address: string) => Promise<void> } = {
    "AssetPercentsChange": async (db: PoolClient, event: { [key: string]: any }, address: string): Promise<void> => {
        let res = db.query("INSERT INTO multipool_assets(address, ideal_share)\
            VALUES($2, $1)\
            ON CONFLICT(address) DO UPDATE SET\
        ideal_share = $1; ", [event.percent, event.asset]);
        console.log(res);
    },
    "AssetQuantityChange": async (db: PoolClient, event: { [key: string]: any }, address: string): Promise<void> => {
        let res = db.query("UPDATE multipool_assets SET quantity = $1 WHERE address = $2", [event.quantity, event.asset]);
        console.log(res);
    },
    "AssetPriceChange": async (db: PoolClient, event: { [key: string]: any }, address: string): Promise<void> => {
        let res = db.query("INSERT INTO multipool_assets(address, price)\
            VALUES($2, $1)\
            ON CONFLICT(address) DO UPDATE SET\
        price = $1; ", [event.price, event.asset]);
        console.log(res);
    },
}
