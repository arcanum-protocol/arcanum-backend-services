import { Pool } from 'pg';

const pool = new Pool({
    connectionString: process.env.DATABASE_URL,
});

const PRICE_AGG_INTERVAL: number = Number(process.env.PRICE_AGG_INTERVAL);

async function aggregate_price(): Promise<void> {
    const client = await pool.connect();

    const indexes = (await client.query('select id from indexes;')).rows.map(v => v.id);

    for (let i = 0; i < indexes.length; i++) {
        const result = await client.query(`call assemble_price(${indexes[i]})`);
        console.log(`aggregating for ${indexes[i]} result ${result}`);
    }
    console.log(`aggregation done`);
}

// run every min
aggregate_price();
setInterval(aggregate_price, PRICE_AGG_INTERVAL);
