import { PoolClient } from "pg";
import { Adatper } from "./adapter";
import { DataLog } from "./log";
import { EventData } from "web3-eth-contract";

class Engine {
    constructor(adapter: Adatper, db: PoolClient) {
        this.adapter = adapter;
        this.db = db;
    }
    adapter: Adatper;
    db: PoolClient

    async process(events: EventData[]): Promise<undefined> {
        for( const event of events) {
            const log = new DataLog(event);
            log.process(this.db);
        }
    }

    async work() {
        
    }

    async update_logs(): Promise<undefined> {
        setInterval(this.work);
    }
}

