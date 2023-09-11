import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL") || "";
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL") || "";

const pool = new Pool(DATABASE_URL, 10);

async function updateRevenueData(): Promise<void> {
    const client = await pool.connect();
    const assets = (await client.queryObject(
        `SELECT defilama_id FROM assets where defilama_id is not null;`,
    )).rows.map((v: any) => v.defilama_id);

    console.log("assets ", assets);

    for (let i = 0; i < assets.length; i++) {
        const defilama_id = assets[i];
        const response = await fetch(
            `https://api.llama.fi/summary/fees/${defilama_id}?dataType=dailyRevenue`,
        );
        const data = await response.json();
        console.log("fetched data");

        console.log("fetched data ", data);
        const revenue = data
            .totalDataChart
            .sort((a: any, b: any) => b[0] - a[0])
            .slice(0, 30)
            .reduce((acc: number, v: any, _i: number, a: any) => (acc + v[1] / a.length), 0);

        await client.queryObject(
            `UPDATE assets SET revenue = $2 where defilama_id = $1`,
            [
                defilama_id,
                revenue,
            ],
        );
        console.log("inserted ", revenue, " to ", defilama_id);
    }
    client.release();
}

await updateRevenueData();
cron(CRON_INTERVAL, async () => {
    await updateRevenueData();
});
