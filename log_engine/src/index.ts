import { Pool } from 'pg';
import { Engine } from './engine';
import { Adatper } from './adapter';
import { ContractConfig } from './types';
import MultiPoolAbi from "./Multipool.json";
import { AbiItem } from 'web3-utils';
import fs from "fs";
//env: RUNNER_ID - id in postgres
//env: CONTRACT_ADDRESS - address of indexing contract
//env: DATABASE_URL
//env: PROVIDER_URL - rpc node id

async function main() {
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
