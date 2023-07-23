import { ABI } from "./multipool-abi.ts";
import Web3 from "npm:web3";
import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import { Lock } from "https://deno.land/x/async@v2.0.2/lock.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;

const pool = new Pool(DATABASE_URL, 10);

async function process() {
    let client = await pool.connect();
    const res = await client.queryObject(
        "SELECT \
            ma.asset_address, \
            m.rpc_url \
        FROM multipool_assets ma \
            JOIN multipools m on m.address = ma.multipool_address\
        WHERE ma.asset_symbol IS NULL; "
    );
    console.log(res.rows);
    let unknownTokens = res.rows;
    for (let i = 0; i < unknownTokens.length; i++) {
        await LOCK.lock(async () => {
            const token = unknownTokens[i];
            const web3 = new Web3(token.rpc_url);
            const contract = new web3.eth.Contract(ABI, token.asset_address);
            const symbol = await contract.methods.symbol().call();
            const decimals = await contract.methods.decimals().call();
            console.log(`Fetched symbol: ${symbol} for ${token.asset_address}`);
            await client.queryObject(
                "UPDATE multipool_assets SET asset_symbol=$1, decimals=$2 where asset_address=$3",
                [symbol, decimals, token.asset_address],
            );
        });
    }
    client.release();
}

const LOCK = new Lock({});
cron(CRON_INTERVAL, async () => {
    await process();
});
