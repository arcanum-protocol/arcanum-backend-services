"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Engine = void 0;
const pg_1 = require("pg");
const log_1 = require("./log");
class Engine {
    constructor(adapter, db, id) {
        this.adapter = adapter;
        this.pool = db;
        this.worker_id = id;
        console.log(db);
    }
    store_data(events, client) {
        return __awaiter(this, void 0, void 0, function* () {
            try {
                yield client.query('BEGIN');
                for (const event of events) {
                    // string to enum conversion
                    const process = log_1.assemble_log[event.event];
                    // check if we handle this event
                    if (process) {
                        yield process(client, event.returnValues, event.address);
                    }
                }
            }
            catch (e) {
                yield client.query('ROLLBACK');
                throw e;
            }
        });
    }
    work() {
        return __awaiter(this, void 0, void 0, function* () {
            yield this.update_logs();
            setInterval(this.update_logs, 60000);
        });
    }
    update_logs() {
        return __awaiter(this, void 0, void 0, function* () {
            const pool = new pg_1.Pool({
                connectionString: this.pool,
            });
            const client = yield pool.connect();
            let height = yield client.query('SELECT block_height FROM indexers_height WHERE id = $1', [this.worker_id]);
            console.log(height);
            if (height.rows.length == 0) {
                throw 'no indexer found';
            }
            ;
            const last_height = height.rows[0].block_height;
            const current_height = yield this.adapter.get_height();
            const logs = yield this.adapter.fetch_logs(last_height, current_height);
            console.log(logs);
            yield this.store_data(logs, client);
            yield client.query('UPDATE indexers_height SET block_height = $1 WHERE id = $2', [current_height, this.worker_id]);
            yield client.query('COMMIT');
            client.release();
        });
    }
}
exports.Engine = Engine;
