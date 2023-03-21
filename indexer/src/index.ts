import { Pool } from 'pg';
import { BigNumber } from 'bignumber.js';

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

interface Asset {
  symbol: string;
  current_price: string;
  mcap: string;
}

async function getAssetsData(coins: Array<string>): Promise<Array<Asset>> {
  const response = await fetch(`https://api.coingecko.com/api/v3/simple/price?ids=${coins.join(',')}&vs_currencies=usd&include_market_cap=true`);
  const data = await response.json();
  let results: Array<Asset> = [];
  for (const coin in data) {
    results.push({
      symbol: coin,
      current_price: new BigNumber(data[coin].usd).toString(10),
      mcap: new BigNumber(data[coin].usd_market_cap).toString(10).split('.')[0],
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
      await client.query('UPDATE assets SET price = $1, mcap = $2 WHERE coingecko_id = $3',
        [asset.current_price, asset.mcap, asset.symbol]);
    }
  } catch (e) {
    throw e;
  } finally {
    client.release();
  }
  console.log('batch updated');
}

// run every min
updateIndex();
setInterval(updateIndex, 60000);
