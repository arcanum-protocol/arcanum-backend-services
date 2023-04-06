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
const engine_1 = require("./engine");
const adapter_1 = require("./adapter");
const Multipool_json_1 = __importDefault(require("./Multipool.json"));
//env: RUNNER_ID - id in postgres
//env: CONTRACT_ADDRESS - address of indexing contract
//env: DATABASE_URL
//env: PROVIDER_URL - rpc node id
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        const id = process.env.RUNNER_ID ? +process.env.RUNNER_ID : 0;
        function configs_from_env() {
            return {
                address: process.env.CONTRACT_ADDRESS || "",
                provider_url: process.env.PROVIDER_URL || "",
                abi: Multipool_json_1.default
            };
        }
        const adapter = new adapter_1.Adatper(configs_from_env(), null);
        const engine = new engine_1.Engine(adapter, process.env.DATABASE_URL || "", id);
        engine.work();
    });
}
main().then();
