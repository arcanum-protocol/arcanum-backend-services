import { Pool } from 'pg';
import cron from 'node-cron';

const pool = new Pool({
    connectionString: process.env.DATABASE_URL,
});

const CRON_INTERVAL: string = String(process.env.CRON_INTERVAL);

async function aggregate_price(): Promise<void> {
    const client = await pool.connect();

    const indexes = (await client.query('select id from indexes;')).rows.map(v => v.id);

    for (let i = 0; i < indexes.length; i++) {
        console.log(`start aggregating for ${indexes[i]}`);
        const result = await client.query(`call assemble_price(${indexes[i]})`);
        console.log(`aggregated for ${indexes[i]} result ${JSON.stringify(result)}`);
    }
    console.log(`aggregation done`);
}

cron.schedule(CRON_INTERVAL, function() {
    aggregate_price();
});
