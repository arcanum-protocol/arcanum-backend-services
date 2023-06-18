import { Pool } from 'pg';
import { BigNumber } from 'bignumber.js';
import cron from 'node-cron';

const pool = new Pool({
    connectionString: process.env.DATABASE_URL,
});

const CRON_INTERVAL: string = String(process.env.CRON_INTERVAL);

interface Asset {
    symbol: string;
    current_price: string;
    market_cap: string;
    volume_24h: string;
    change_24h: string;
}

async function getAssetsData(coins: Array<string>): Promise<Array<Asset>> {
    const response = await fetch(`https://api.coingecko.com/api/v3/simple/price?ids=${coins.join(',')}&vs_currencies=usd&include_market_cap=true&include_24hr_vol=true&include_24hr_change=true&precision=6`);
    const data = await response.json();
    let results: Array<Asset> = [];
    for (const coin in data) {
        results.push({
            symbol: coin,
            current_price: new BigNumber(data[coin].usd).toString(10),
            market_cap: new BigNumber(data[coin].usd_market_cap).toString(10),
            volume_24h: new BigNumber(data[coin].usd_24h_vol).toString(10),
            change_24h: new BigNumber(data[coin].usd_24h_change).toString(10),
        });
    }
    console.log(results);
    return results;
}

async function updateIndex(): Promise<void> {
    const client = await pool.connect();
    // get all assets names
    const result = await client.query('SELECT coingecko_id FROM assets');
    const assets = result.rows.map(row => row.coingecko_id);
    // batch request to coingecko, ask price + mcap
    const assetsData = await getAssetsData(assets);
    // update assets table
    try {
        for (let i = 0; i < assetsData.length; i++) {
            const asset = assetsData[i];
            await client.query('UPDATE assets SET price = $1, mcap = $2, volume_24h = $4, price_change_24h = $5  WHERE coingecko_id = $3',
                [asset.current_price, asset.market_cap, asset.symbol, asset.volume_24h, asset.change_24h]);
        }
    } catch (e) {
        throw e;
    } finally {
        client.release();
    }
    console.log('batch updated');
}

cron.schedule(CRON_INTERVAL, function() {
    updateIndex();
});
