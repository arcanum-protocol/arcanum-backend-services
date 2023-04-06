import { Config, ContractConfig, BlockNumber } from "./types";
import { Contract, EventData } from "web3-eth-contract";
import Web3 from "web3";

export class Adatper {

    constructor(contract_config: ContractConfig, config: Config | null) {
        const web3 = new Web3(contract_config.provider_url);
        console.log(contract_config.provider_url);
        
        this.contract = new web3.eth.Contract(contract_config.abi, contract_config.address);
        this.event = config?.event || "allEvents";
        this.provider_url = contract_config.provider_url;
        this.web3 = new Web3(this.provider_url);
    }

    provider_url: string;
    web3: Web3;
    event: string;
    contract: Contract;

    // we take strict blocks so we don't skip any of them between queries
    async fetch_logs(from_block: BlockNumber, to_block: BlockNumber): Promise<EventData[]> {
        const res = await this.contract.getPastEvents(this.event, {
            fromBlock: from_block,
            toBlock: to_block,
        });
        return res;
    } 

    async get_height(): Promise<BlockNumber> {
        return (await this.web3.eth.getBlock("latest")).number;
    }
}