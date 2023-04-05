import { PoolClient, Pool } from "pg";
import { Adatper } from "./adapter";
import { EventType } from "./log"
import { EventData } from "web3-eth-contract";
import { assemble_log } from "./log";

export class Engine {
    constructor(adapter: Adatper, db: string, id: number) {
        this.adapter = adapter;
        this.pool = db;
        this.worker_id = id;
        console.log(db);
    }

    worker_id: number;
    adapter: Adatper;
    pool: string

    async store_data(events: EventData[], client: PoolClient): Promise<void> {
        try {
            await client.query('BEGIN')
            for( const event of events) {
                // string to enum conversion
                const process = assemble_log[event.event as EventType];
                // check if we handle this event
                if (process) {
                    await process(client, event.returnValues, event.address);

                }
            }
        } catch(e) {
            await client.query('ROLLBACK');
            throw e;
        }
    }

    async work() {
        await this.update_logs();
        setInterval(this.update_logs, 60000);
    }

    async update_logs(): Promise<void> {
        const pool = new Pool({
            connectionString: this.pool,
          });
        const client = await pool.connect();
        let height = await client.query('SELECT block_height FROM indexers_height WHERE id = $1', [this.worker_id]);
        console.log(height);
        const block_height = await this.adapter.get_height() - 1000;
        if (height.rows.length == 0) {
            client.query("INSERT INTO indexers_height(id, block_height) VALUES ($1, $2)", [this.worker_id, block_height]);
        };
        const last_height = height.rows[0] ? height.rows[0].block_height : block_height;
        const current_height = await this.adapter.get_height();
        const logs = await this.adapter.fetch_logs(last_height, current_height);
        console.log(logs);
        await this.store_data(logs, client);
        console.log(12);
        
        await client.query('UPDATE indexers_height SET block_height = $1 WHERE id = $2', [current_height, this.worker_id]);
        client.release();
    }
}

