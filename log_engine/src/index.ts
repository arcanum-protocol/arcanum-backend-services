import { Pool } from 'pg';
import { Engine } from './engine';
import { Adatper } from './adapter';
import { ContractConfig } from './types';
import MultiPoolAbi from "./Multipool.json";
import { AbiItem } from 'web3-utils';
import fs from "fs";

async function main() {
  const pool = new Pool({
    connectionString: process.env.DATABASE_URL,
  });

  const client = await pool.connect();
  var sql = fs.readFileSync("./postgress/log_and_indexers.sql", "utf8");
  await client.query(sql);
  const id = process.env.RUNNER_ID ? +process.env.RUNNER_ID : 0;
  
  function configs_from_env(): ContractConfig {
    return {
      address: process.env.CONTRACT_ADDRESS || "",
      provider_url: process.env.PROVIDER_URL || "",
      abi: MultiPoolAbi as AbiItem[]
    }
  }
  
  const adapter = new Adatper(configs_from_env(), null);
  const engine = new Engine(adapter, process.env.DATABASE_URL || "", id);
  engine.work();
}

main().then();