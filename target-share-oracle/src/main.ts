import { ABI } from "./multipool-abi.ts";
import { ethers } from "npm:ethers@5.7.0";
import { Pool, PoolClient } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import { Lock } from "https://deno.land/x/async@v2.0.2/lock.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;
const PRIVATE_KEY = Deno.env.get("PRIVATE_KEY")!;

const pool = new Pool(DATABASE_URL, 10);

// Define function to get current price for a single token from Postgres
async function getTokenPriceAndMarketCap(
    client: PoolClient,
    assetAddress: string,
    multipoolAddress: string,
): Promise<[string, string]> {
    const res = await client.queryObject(
        "SELECT price, mcap FROM assets \
        WHERE symbol = (select asset_symbol from multipool_assets where asset_address = $1 and multipool_address=$2);",
        [assetAddress, multipoolAddress],
    );
    const row: any = res.rows[0];
    return [row.price, row.mcap];
}

async function process() {
    console.log("starting processing");
    const client = await pool.connect();
    let multipools = await client.queryObject(
        "SELECT rpc_url, address FROM multipools where testnet = true;"
    );
    console.log(multipools.rows);

    for (let i = 0; i < multipools.rows.length; i++) {
        const multipool = multipools.rows[i];
        await updateAllTokenPrices(multipool.rpc_url, multipool.address);
    }

    client.release();
}

// Define function to update prices for all tokens in the database
async function updateAllTokenPrices(
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
        let prices: string[] = [];
        for (let i = 0; i < addresses.rows.length; i++) {
            const assetAddress = addresses.rows[i].asset_address;
            const [newPrice, newPercents] = await getTokenPriceAndMarketCap(
                client,
                assetAddress,
                multipoolAddress,
            );
            // convert price to 18 decimal places
            assets.push(assetAddress);
            const newPrice18 = ethers.utils.parseEther(newPrice).toString();
            prices.push(newPrice18);

            // console.log(`updating price for ${assetAddress} to ${newPrice18} `);
            // const tx = await contract.updatePrices([assetAddress], [newPrice18]);
            // console.log(`Transaction sent: ${tx.hash}`);
            // // Wait for transaction to be confirmed
            // const receipt = await tx.wait();
            // console.log(`Transaction confirmed in block ${receipt.blockNumber}`);
        }
        console.log(`updating price for ${assets} to ${prices} `);
        const tx = await contract.updatePrices(assets, prices);
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
