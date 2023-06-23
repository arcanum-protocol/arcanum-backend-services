import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL") || "";
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL") || "";

const pool = new Pool(DATABASE_URL, 10);

async function aggregate_price(): Promise<void> {
    const client = await pool.connect();

    const indexes = (await client.queryObject("select id from indexes;")).rows
        .map(
            (v: any) => v.id,
        );

    for (let i = 0; i < indexes.length; i++) {
        console.log(`start aggregating for ${indexes[i]}`);
        const result = await client.queryObject(
            `call assemble_price(${indexes[i]})`,
        );
        console.log(
            `aggregated for ${indexes[i]} result ${JSON.stringify(result)}`,
        );
    }
    console.log(`aggregation done`);
    client.release();
}

cron(CRON_INTERVAL, async () => {
    await aggregate_price();
});
