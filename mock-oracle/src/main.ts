import { ABI } from "./multipool-abi.ts";
import { ethers } from "npm:ethers@5.7.0";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import { Lock } from "https://deno.land/x/async@v2.0.2/lock.ts";
import Redstone from "npm:redstone-api@0.4.11";
import Yaml from "npm:js-yaml@4.1.0";

const PRICE_ORACLE_ID = Deno.env.get("PRICE_ORACLE_ID")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;
const PRIVATE_KEY = Deno.env.get("PRIVATE_KEY")!;
const SCHEME_PATH = Deno.env.get("SCHEME")!;

const decoder = new TextDecoder("utf-8");
console.log(SCHEME_PATH);
const SCHEME = Yaml.load(decoder.decode(Deno.readFileSync(SCHEME_PATH)));

async function fetchCoingecko(assets: any) {
    const response = await fetch(
        `https://api.coingecko.com/api/v3/simple/price?ids=${assets.map((v: any) => v.coingecko_id).join(",")
        }&vs_currencies=usd&include_market_cap=true&include_24hr_vol=true&include_24hr_change=true`,
    );
    const data: any = await response.json();
    console.log(data);
    return assets.map((asset: any) => {
        asset.price = ethers.utils.parseEther(data[asset.coingecko_id].usd.toString()).toString();
        return asset;
    });
}

async function fetchRedstone(assets: any) {
    const data = await Redstone.getPrice(assets.map((v: any) => v.symbol));
    console.log(data);
    return assets.map((asset: any) => {
        asset.price = ethers.utils.parseEther(data[asset.symbol].value.toString()).toString();
        return asset;
    });
}

async function process() {
    console.log("start processing");
    Object
        .entries(SCHEME)
        .filter(([_multipoool_id, multipool]: [string, any]) => multipool.price_oracle_id == PRICE_ORACLE_ID)
        .forEach(async ([multipool_id, multipool]: [string, any]) => {
            const provider = new ethers.providers.JsonRpcProvider(multipool.rpc_url);
            const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
            const contract = new ethers.Contract(multipool.address, ABI, wallet);

            await LOCK.lock(async () => {
                let assets: string[] = [];
                let prices: string[] = [];

                const gecko_origins = multipool
                    .assets
                    .filter((asset: any) => asset.price_origin.name == "gecko")
                    .map((asset: any) => {
                        return {
                            address: asset.address,
                            coingecko_id: asset.price_origin.coingecko_id,
                        };
                    });
                const gecko_feeds = await fetchCoingecko(gecko_origins);
                assets = assets.concat(gecko_feeds.map(v => v.address));
                prices = prices.concat(gecko_feeds.map(v => v.price));

                const redstone_origins = multipool
                    .assets
                    .filter((asset: any) => asset.price_origin.name == "redstone")
                    .map((asset: any) => {
                        return {
                            address: asset.address,
                            symbol: asset.price_origin.symbol,
                        };
                    });
                const redstone_feeds = await fetchRedstone(redstone_origins);
                assets = assets.concat(redstone_feeds.map(v => v.address));
                prices = prices.concat(redstone_feeds.map(v => v.price));

                console.log(`updating ${multipool_id} price for ${assets} to ${prices} `);
                const tx = await contract.updatePrices(assets, prices);
                console.log(`Transaction sent: ${tx.hash}`);
                const receipt = await tx.wait();
                console.log(`Transaction confirmed in block ${receipt.blockNumber}`);
            });
        });
}

const LOCK = new Lock({});
await process();
cron(CRON_INTERVAL, async () => {
    await process();
});
