import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import { cron } from "https://deno.land/x/deno_cron/cron.ts";

const DATABASE_URL = Deno.env.get("DATABASE_URL") || "";
const CRON_INTERVAL = Deno.env.get("CRON_INTERVAL") || "";

const pool = new Pool(DATABASE_URL, 10);

async function updateRevenueData(): Promise<void> {
  const response = await fetch(
    "https://api.llama.fi/overview/fees/arbitrum?excludeTotalDataChartBreakdown=true&excludeTotalDataChart=true",
  );
  const data = await response.json();
  console.log("fetched data");
  const protocols = data["protocols"];

  const client = await pool.connect();
  const assets = (await client.queryObject(
    `SELECT defilama_id FROM assets where defilama_id is not null;`,
  )).rows.map((v: any) => v.defilama_id);

  console.log("assets ", assets);

  for (let i = 0; i < protocols.length; i++) {
    const id = protocols[i]["defillamaId"];
    const revenue = protocols[i]["dailyHoldersRevenue"];

    if (assets.indexOf(id) === -1) {
      continue;
    }
    await client.queryObject(
      `UPDATE assets SET revenue = $2 where defilama_id = $1`,
      [
        id,
        revenue,
      ],
    );
    console.log("inserted ", revenue, " to ", id);
  }
  client.release();
}

cron(CRON_INTERVAL, async () => {
  await updateRevenueData();
});
