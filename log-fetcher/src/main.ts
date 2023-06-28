import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import Web3 from "npm:web3";
import { ABI } from "./multipool-abi.ts";
import { Lock } from "https://deno.land/x/async@v2.0.2/lock.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;
const CONTRACT_ADDRESS = Deno.env.get("CONTRACT_ADDRESS")!.toLowerCase();
const RUNNER_ID = Deno.env.get("RUNNER_ID")!;
const PROVIDER_URL = Deno.env.get("PROVIDER_URL")!;
const MAXIMUM_BLOCKS_PER_REQUEST: bigint = BigInt(
    Deno.env.get("MAXIMUM_BLOCKS_PER_REQUEST") || 1000,
);

const pool = new Pool(DATABASE_URL, 10);

const web3 = new Web3(PROVIDER_URL);
const contract = new web3.eth.Contract(ABI, CONTRACT_ADDRESS);

async function getEvents() {
    const client = await pool.connect();

    let last_block: bigint = await client.queryObject(
        "SELECT block_height FROM indexers_height WHERE id = $1",
        [RUNNER_ID],
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
                    "INSERT INTO etf_assets(multipool_address, asset_address, ideal_share)\
                VALUES($3, $2, $1)\
                ON CONFLICT(multipool_address, asset_address) DO UPDATE SET\
                ideal_share = $1;",
                    [values.percent, values.asset, log.address.toLowerCase()],
                );
                console.log(res);
            } else if (log.event == "AssetQuantityChange") {
                const res = await client.queryObject(
                    "UPDATE etf_assets SET quantity = $1 WHERE multipool_address = $3 and asset_address = $2;",
                    [values.quantity, values.asset, log.address.toLowerCase()],
                );
                console.log(res);
            } else if (log.event == "AssetPriceChange") {
                const res = await client.queryObject(
                    "INSERT INTO etf_assets(multipool_address, asset_address, chain_price)\
                VALUES($3, $2, $1)\
                ON CONFLICT(multipool_address, asset_address) DO UPDATE SET\
                chain_price = $1;",
                    [values.price, values.asset, log.address.toLowerCase()],
                );
                console.log(res);
            }
        });
    });

    await client.queryObject(
        "UPDATE indexers_height SET block_height = $1 WHERE id = $2",
        [current_block, RUNNER_ID],
    );
    await client.queryObject("COMMIT;");
    client.release();
    console.log("finised iteration");
}

const LOCK = new Lock({});
const TRANSACTION_LOCK = new Lock({});
cron(CRON_INTERVAL, async () => {
    await LOCK.lock(async () => {
        await getEvents();
    });
});
