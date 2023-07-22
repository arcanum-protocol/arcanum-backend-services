import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import Web3 from "npm:web3";
import { ABI } from "./multipool-abi.ts";
import { Lock } from "https://deno.land/x/async@v2.0.2/lock.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;
const MAXIMUM_BLOCKS_PER_REQUEST: bigint = BigInt(
    Deno.env.get("MAXIMUM_BLOCKS_PER_REQUEST") || 1000,
);

const pool = new Pool(DATABASE_URL, 10);

async function process() {
    console.log("starting processing");
    const client = await pool.connect();
    let multipools = await client.queryObject(
        "SELECT rpc_url, address FROM multipools;"
    );
    console.log(multipools.rows);

    for (let i = 0; i < multipools.rows.length; i++) {
        const multipool = multipools.rows[i];
        await getEvents(multipool.rpc_url, multipool.address);
    }

    client.release();
}

async function getEvents(rpc_url: string, multipool_address: string) {
    const client = await pool.connect();

    const web3 = new Web3(rpc_url);
    const contract = new web3.eth.Contract(ABI, multipool_address);

    let last_block: bigint = await client.queryObject(
        "SELECT block_height FROM multipools WHERE address=$1",
        [multipool_address],
    ).then((v: any) => BigInt(v.rows[0].block_height))!;

    let current_block = await web3
        .eth
        .getBlock("latest")
        .then((v: any) => BigInt(v.number));

    console.log(`from block ${last_block}`);
    if (current_block - last_block > MAXIMUM_BLOCKS_PER_REQUEST) {
        current_block = last_block + MAXIMUM_BLOCKS_PER_REQUEST;
    }

    const logs = await contract.getPastEvents("allEvents", {
        fromBlock: last_block,
        toBlock: current_block,
    });

    await client.queryObject("BEGIN;");

    console.log(logs);
    await logs.forEach(async (log: any) => {
        await TRANSACTION_LOCK.lock(async () => {
            const values = log.returnValues;
            if (log.event == "AssetPercentsChange") {
                const res = await client.queryObject(
                    "INSERT INTO multipool_assets(multipool_address, asset_address, ideal_share)\
                VALUES($3, $2, $1)\
                ON CONFLICT(multipool_address, asset_address) DO UPDATE SET\
                ideal_share = $1;",
                    [values.percent, values.asset.toLowerCase(), log.address.toLowerCase()],
                );
                console.log(res);
            } else if (log.event == "AssetQuantityChange") {
                const res = await client.queryObject(
                    "UPDATE multipool_assets SET quantity = $1 WHERE multipool_address = $3 and asset_address = $2;",
                    [values.quantity, values.asset.toLowerCase(), log.address.toLowerCase()],
                );
                console.log(res);
            } else if (log.event == "AssetPriceChange") {
                const res = await client.queryObject(
                    "INSERT INTO multipool_assets(multipool_address, asset_address, chain_price)\
                VALUES($3, $2, $1)\
                ON CONFLICT(multipool_address, asset_address) DO UPDATE SET\
                chain_price = $1;",
                    [values.price, values.asset.toLowerCase(), log.address.toLowerCase()],
                );
                console.log(res);
            } else if (log.event == "Transfer") {
                if (values.to == "0x0000000000000000000000000000000000000000") {
                    const res = await client.queryObject(
                        "UPDATE multipools SET total_supply=total_supply-$1 WHERE address=$2;",
                        [values.value, log.address.toLowerCase()],
                    );
                    console.log(res);
                } else if (values.from == "0x0000000000000000000000000000000000000000") {
                    const res = await client.queryObject(
                        "UPDATE multipools SET total_supply=total_supply+$1 WHERE address=$2;",
                        [values.value, log.address.toLowerCase()],
                    );
                    console.log(res);
                }
            }
        });
    });

    await client.queryObject(
        "UPDATE multipools SET block_height = $1 WHERE address = $2",
        [current_block, multipool_address],
    );
    await client.queryObject("COMMIT;");
    client.release();
    console.log("finised iteration");
}

const LOCK = new Lock({});
const TRANSACTION_LOCK = new Lock({});
cron(CRON_INTERVAL, async () => {
    await LOCK.lock(async () => {
        await process();
    });
});
