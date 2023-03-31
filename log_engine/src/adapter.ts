import { Config, ContractConfig, BlockNumber } from "./types";
import { Contract, EventData } from "web3-eth-contract";
import Web3 from "web3";

export class Adatper {

    constructor(config: Config | null, contract_config: ContractConfig) {
        Contract.setProvider(contract_config.provider_url);
        this.contract = new Contract(contract_config.abi, contract_config.address);
        this.event = config?.event || "allEvents";
        this.provider_url = contract_config.provider_url;
        this.web3 = new Web3(this.provider_url);
    }

    provider_url: string;
    web3: Web3;
    event: string;
    contract: Contract;

    async fetch_logs(from_block: BlockNumber): Promise<EventData[]> {
        const res = await this.contract.getPastEvents(this.event, {
            fromBlock: from_block,
            toBlock: "latest",
        });
        return res;
    } 

    async get_height(): Promise<BlockNumber> {
        return (await this.web3.eth.getBlock("latest")).number;
    }
}