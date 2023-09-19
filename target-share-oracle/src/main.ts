import { ABI } from "./multipool-abi.ts";
import { ethers } from "npm:ethers@5.7.0";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";
import { Lock } from "https://deno.land/x/async@v2.0.2/lock.ts";
import Yaml from "npm:js-yaml@4.1.0";

const TARGET_SHARE_ORACLE_ID = Deno.env.get("TARGET_SHARE_ORACLE_ID")!;
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL")!;
const PRIVATE_KEY = Deno.env.get("PRIVATE_KEY")!;
const SCHEME_PATH = Deno.env.get("SCHEME")!;

const MAX_SHARE = Deno.env.get("MAX_SHARE")!;

const decoder = new TextDecoder("utf-8");
console.log(SCHEME_PATH);
const SCHEME = Yaml.load(decoder.decode(Deno.readFileSync(SCHEME_PATH)));

async function fetchDefillama(assets: any) {
    return await Promise.all(assets.map(async asset => {
        const response = await fetch(
            `https://api.llama.fi/summary/fees/${asset.defillama_id}?dataType=dailyRevenue`,
        );
        const data = await response.json();
        console.log("fetched data ", data);
        asset.revenue = data
            .totalDataChart
            .sort((a: any, b: any) => b[0] - a[0])
            .slice(0, 30)
            .reduce((acc: number, v: any, _i: number, a: any) => (acc + v[1] / a.length), 0);
        asset.revenue = BigInt(ethers.utils.parseEther(asset.revenue.toString()).toString());
        return asset;
    }));
}

async function process() {
    console.log("start processing");
    Object
        .entries(SCHEME)
        .filter(([_multipoool_id, multipool]: [string, any]) => multipool.target_share_oracle_id == TARGET_SHARE_ORACLE_ID)
        .forEach(async ([multipool_id, multipool]: [string, any]) => {
            const provider = new ethers.providers.JsonRpcProvider(multipool.rpc_url);
            const wallet = new ethers.Wallet(PRIVATE_KEY, provider);
            const contract = new ethers.Contract(multipool.address, ABI, wallet);

            await LOCK.lock(async () => {
                const origins = multipool
                    .assets
                    .map((asset: any) => {
                        return {
                            address: asset.address,
                            defillama_id: asset.defillama_id,
                        };
                    });
                let revenue_feeds = await fetchDefillama(origins);
                console.log(revenue_feeds);

                const avg = revenue_feeds.reduce((
                    acc: BigInt,
                    v: { address: string, revenue: bigint },
                    _i: number,
                    a: any
                ) => (BigInt(acc.toString()) + BigInt(v.revenue) / BigInt(a.length)), BigInt(0))

                const maxShare = BigInt(ethers.utils.parseEther(MAX_SHARE).toString());
                const ONE = BigInt(ethers.utils.parseEther('1').toString());
                revenue_feeds = revenue_feeds.map((val: { address: string, revenue: bigint }) => {
                    if (val.revenue > avg * (ONE + maxShare) / ONE) {
                        return { address: val.address, revenue: avg * (ONE + maxShare) / ONE }
                    } else if (val.revenue < avg * (ONE - maxShare) / ONE) {
                        return { address: val.address, revenue: avg * (ONE - maxShare) / ONE }
                    } else {
                        return val
                    }
                });

                const assets = revenue_feeds.map((v) => v.address);
                const revenues = revenue_feeds.map((v) => v.revenue);


                console.log(`updating ${multipool_id} price for ${assets} to ${revenues} `);
                const tx = await contract.updateTargetShares(assets, revenues);
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
