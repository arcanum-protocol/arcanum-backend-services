import { Pool } from 'pg';
import cron from 'node-cron';

const pool = new Pool({
    connectionString: process.env.DATABASE_URL,
});

const CRON_INTERVAL: string = String(process.env.CRON_INTERVAL);

async function updateRevenueData(): Promise<void> {
    const response = await fetch('https://api.llama.fi/overview/fees/arbitrum?excludeTotalDataChartBreakdown=true&excludeTotalDataChart=true');
    let data = await response.json();
    console.log("fetched data");
    const protocols = data["protocols"];

    const client = await pool.connect();
    const assets = (await client.query(`SELECT defilama_id FROM assets where defilama_id is not null;`)).rows.map(v => v.defilama_id);

    console.log("assets ", assets);

    for (let i = 0; i < protocols.length; i++) {
        let id = protocols[i]["defillamaId"];
        let revenue = protocols[i]["dailyHoldersRevenue"];

        if (assets.indexOf(id) === -1) {
            continue;
        }
        await pool.query(`UPDATE assets SET revenue = $2 where defilama_id = $1`, [id, revenue]);
        console.log("inserted ", revenue, " to ", id);

    }
}

cron.schedule(CRON_INTERVAL, function() {
    updateRevenueData();
});
