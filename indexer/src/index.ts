import { Pool } from 'pg';
import { searchPairsMatchingQuery } from 'dexscreener-api';
import Pair from 'dexscreener-api/dist/types/Pair';

interface Asset {
  symbol: string;
  name: string;
  mcap: number;
  address: string;
  price: number;
}

// usd stables with name that doesn't match `USD`
const excludeCoins = [
  'dai',
  'drax',
  'paxg',
  'xaut',
  'ust',
  'mim',
  'eur',
  'dola'
]

async function getTop50AssetsByMcapWithLiquidity(): Promise<Array<Asset>> {
  let top100AssetsByMcap = await fetch('https://api.coingecko.com/api/v3/coins/markets?vs_currency=usd&order=market_cap_desc&per_page=140&page=1&sparkline=false')
      .then(response => response.json())
      .catch(error => console.error(error));
  // remove all coins that contains `W` at the start
  top100AssetsByMcap = top100AssetsByMcap.filter(asset => !asset.symbol.startsWith('w'));
  // remove all coins that contains `USD` in the name
  top100AssetsByMcap = top100AssetsByMcap.filter(asset => !asset.symbol.includes('usd'));
  // remove usd stables
  top100AssetsByMcap = top100AssetsByMcap.filter(asset => !excludeCoins.includes(asset.symbol));

  let assetsWithLiquidity: Array<Asset> = await Promise.all(top100AssetsByMcap.map(async (asset: any) => {
      const symbol = asset.symbol.toUpperCase();
      let dexScreenerResults: Array<Pair> = [];

      dexScreenerResults = (await searchPairsMatchingQuery(symbol)).pairs;

      let EthLiquidity: Array<Pair> = dexScreenerResults?.filter(pair =>
          pair?.chainId === 'ethereum' &&
          (Number(pair?.priceUsd) > (asset.current_price * 0.9) && Number(pair?.priceUsd) < (asset.current_price * 1.1)) &&
          ((pair?.baseToken?.symbol === symbol || pair?.quoteToken?.symbol === symbol) ||
              (pair?.baseToken?.symbol === `W${symbol}` || pair?.quoteToken?.symbol === `W${symbol}`))
      );

      if (EthLiquidity.length === 0) {
          dexScreenerResults = (await searchPairsMatchingQuery(`W${symbol}`)).pairs;
          EthLiquidity = dexScreenerResults?.filter(pair =>
              pair?.chainId === 'ethereum' &&
              (Number(pair?.priceUsd) > (asset.current_price * 0.9) && Number(pair?.priceUsd) < (asset.current_price * 1.1)) &&
              ((pair?.baseToken?.symbol === symbol || pair?.quoteToken?.symbol === symbol) ||
                  (pair?.baseToken?.symbol === `W${symbol}` || pair?.quoteToken?.symbol === `W${symbol}`))
          );
      }

      if (EthLiquidity.length === 0) {
          dexScreenerResults = (await searchPairsMatchingQuery(`W${symbol} weth`)).pairs;
          EthLiquidity = dexScreenerResults?.filter(pair =>
              pair?.chainId === 'ethereum' &&
              (Number(pair?.priceUsd) > (asset.current_price * 0.9) && Number(pair?.priceUsd) < (asset.current_price * 1.1)) &&
              ((pair?.baseToken?.symbol === symbol || pair?.quoteToken?.symbol === symbol) ||
                  (pair?.baseToken?.symbol === `W${symbol}` || pair?.quoteToken?.symbol === `W${symbol}`))
          );
      }

      if (EthLiquidity.length === 0) {
          return null;
      }

      // find the pair that matches the symbol, and +-1% to the coingecko price
      // then return the address
      const pair = EthLiquidity[0];

      let tokenAddress = (pair?.baseToken.symbol === symbol || pair?.baseToken.symbol === `W${symbol}`) ? pair?.baseToken.address : pair?.quoteToken.address;

      return {
          symbol: symbol,
          name: asset.name,
          mcap: asset.market_cap,
          address: tokenAddress,
          current_price: asset.current_price
      };
  }));

  // remove nulls
  assetsWithLiquidity = assetsWithLiquidity.filter(asset => asset !== null);

  const assetsWithLiquiditySortedByMcap = assetsWithLiquidity.sort((a, b) => b.mcap - a.mcap);

  const top50AssetsWithLiquidity = assetsWithLiquiditySortedByMcap.filter(asset => asset.mcap).slice(0, 50);
  return top50AssetsWithLiquidity;
}


async function updateIndex() {
  // get top 50 assets by mcap with liquidity
  let assets: Array<Asset> = [];

  try {
    assets = await getTop50AssetsByMcapWithLiquidity();
  } catch (e) {
    console.error(e);
  }
  // insert price, mcap, by name to db, to `assets` table
  const pool = new Pool({
    user: 'postgres',
    host: 'localhost',
    database: 'postgres',
    password: 'mysecretpassword',
    port: 5432,
  });

  const client = await pool.connect();

  try {
    await client.query('BEGIN');
    for (let i = 0; i < assets.length; i++) {
      const asset = assets[i];
      console.log(asset);
      await client.query('INSERT INTO assets (id, name, price, address, mcap) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO UPDATE SET price = $3, mcap = $5', [i, asset.symbol, asset.current_price, asset.address, asset.mcap]);
      // sleep for 1 second
      await new Promise(resolve => setTimeout(resolve, 250));
    }
    await client.query('COMMIT');
  } catch (e) {
    await client.query('ROLLBACK');
    throw e;
  } finally {
    client.release();
  }
}

// run every 1 minute
setInterval(updateIndex, 60000);