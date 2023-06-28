import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import BigNumber from "npm:bignumber.js@9.0.1";

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;

const pool = new Pool(DATABASE_URL, 10);

interface Asset {
    symbol: string;
    current_price: string;
    market_cap: string;
    volume_24h: string;
    change_24h: string;
}

async function getAssetsData(coins: Array<string>): Promise<Array<Asset>> {
    const response = await fetch(
        `https://api.coingecko.com/api/v3/simple/price?ids=${coins.join(",")
        }&vs_currencies=usd&include_market_cap=true&include_24hr_vol=true&include_24hr_change=true&precision=6`,
    );
    const data = await response.json();
    let results: Array<Asset> = [];
    for (const coin in data) {
        results.push({
            symbol: coin,
            current_price: new BigNumber(data[coin].usd).toString(),
            market_cap: new BigNumber(data[coin].usd_market_cap).toString(),
            volume_24h: new BigNumber(data[coin].usd_24h_vol).toString(),
            change_24h: new BigNumber(data[coin].usd_24h_change).toString(),
        });
    }
    console.log(results);
    return results;
}

async function updateIndex(): Promise<void> {
    const client = await pool.connect();
    // get all assets names
    const result = await client.queryObject("SELECT coingecko_id FROM assets");
    const assets = result.rows.map((row: any) => row.coingecko_id);
    // batch request to coingecko, ask price + mcap
    const assetsData = await getAssetsData(assets);
    // update assets table
    for (let i = 0; i < assetsData.length; i++) {
        const asset = assetsData[i];
        await client.queryObject(
            "UPDATE assets SET price = $1, mcap = $2, volume_24h = $4, price_change_24h = $5  WHERE coingecko_id = $3",
            [
                asset.current_price,
                asset.market_cap,
                asset.symbol,
                asset.volume_24h,
                asset.change_24h,
            ],
        );
    }
    client.release();
    console.log("batch updated");
}

cron(CRON_INTERVAL, async () => {
    await updateIndex();
});
