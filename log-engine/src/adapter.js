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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Adatper = void 0;
const web3_1 = __importDefault(require("web3"));
class Adatper {
    constructor(contract_config, config) {
        const web3 = new web3_1.default(contract_config.provider_url);
        console.log(contract_config.provider_url);
        this.contract = new web3.eth.Contract(contract_config.abi, contract_config.address);
        this.event = (config === null || config === void 0 ? void 0 : config.event) || "allEvents";
        this.provider_url = contract_config.provider_url;
        this.web3 = new web3_1.default(this.provider_url);
    }
    // we take strict blocks so we don't skip any of them between queries
    fetch_logs(from_block, to_block) {
        return __awaiter(this, void 0, void 0, function* () {
            const res = yield this.contract.getPastEvents(this.event, {
                fromBlock: from_block,
                toBlock: to_block,
            });
            return res;
        });
    }
    get_height() {
        return __awaiter(this, void 0, void 0, function* () {
            return (yield this.web3.eth.getBlock("latest")).number;
        });
    }
}
exports.Adatper = Adatper;
