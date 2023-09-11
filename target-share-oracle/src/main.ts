import { ABI } from "./multipool-abi.ts";
import { ethers } from "npm:ethers@5.7.0";
import { Pool, PoolClient } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import { Lock } from "https://deno.land/x/async@v2.0.2/lock.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const TARGET_SHARE_ORACLE_ID = Deno.env.get("TARGET_SHARE_ORACLE_ID")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;
const PRIVATE_KEY = Deno.env.get("PRIVATE_KEY")!;

const pool = new Pool(DATABASE_URL, 10);

// Define function to get current price for a single token from Postgres
async function getTokenRevenue(
    client: PoolClient,
    assetAddress: string,
    multipoolAddress: string,
): Promise<string> {
    const res = await client.queryObject(
        "SELECT revenue FROM assets \
        WHERE symbol = (select asset_symbol from multipool_assets where asset_address = $1 and multipool_address=$2);",
        [assetAddress, multipoolAddress],
    );
    const row: any = res.rows[0];
    return row.revenue;
}

async function process() {
    console.log("starting processing");
    const client = await pool.connect();
    let multipools = await client.queryObject(
        "SELECT rpc_url, address FROM multipools where price_oracle_id = $1;",
        [TARGET_SHARE_ORACLE_ID]
    );
    console.log(multipools.rows);

    for (let i = 0; i < multipools.rows.length; i++) {
        const multipool = multipools.rows[i];
        await updateTargetShares(multipool.rpc_url, multipool.address);
    }

    client.release();
}

// Define function to update prices for all tokens in the database
async function updateTargetShares(
    rpcUrl: string,
    multipoolAddress: string,
) {
    let client = await pool.connect();

    const provider = new ethers.providers.JsonRpcProvider(rpcUrl);
    const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
    // Instantiate contract
    const contract = new ethers.Contract(multipoolAddress, ABI, wallet);

    const addresses: any = await client.queryObject(
        "SELECT asset_address FROM multipool_assets where multipool_address = $1",
        [multipoolAddress],
    );
    // exclude from addresses the address if its persent in contract is zero
    await LOCK.lock(async () => {
        let assets: string[] = [];
        let revenues: string[] = [];
        for (let i = 0; i < addresses.rows.length; i++) {
            const assetAddress = addresses.rows[i].asset_address;
            const revenue = await getTokenRevenue(
                client,
                assetAddress,
                multipoolAddress,
            );
            // convert price to 18 decimal places
            assets.push(assetAddress);
            const newRevenue18 = ethers.utils.parseEther(revenue).toString();
            revenues.push(newRevenue18);
        }
        console.log(`updating price for ${assets} to ${revenues} `);
        const tx = await contract.updateTargetShares(assets, revenues);
        console.log(`Transaction sent: ${tx.hash}`);
        // Wait for transaction to be confirmed
        const receipt = await tx.wait();
        console.log(`Transaction confirmed in block ${receipt.blockNumber}`);
    });
    client.release();
}

const LOCK = new Lock({});
cron(CRON_INTERVAL, async () => {
    await process();
});
