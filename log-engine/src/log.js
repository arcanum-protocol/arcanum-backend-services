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
exports.assemble_log = exports.EventType = void 0;
var EventType;
(function (EventType) {
    EventType["AssetPercentsChange"] = "AssetPercentsChange";
    EventType["AssetQuantityChange"] = "AssetQuantityChange";
    EventType["AssetPriceChange"] = "AssetPriceChange";
})(EventType = exports.EventType || (exports.EventType = {}));
exports.assemble_log = {
    "AssetPercentsChange": (db, event, address) => __awaiter(void 0, void 0, void 0, function* () {
        let res = db.query("INSERT INTO multipool_assets(address, ideal_share)\
            VALUES($2, $1)\
            ON CONFLICT(address) DO UPDATE SET\
        ideal_share = $1; ", [event.percent, event.asset]);
        console.log(res);
    }),
    "AssetQuantityChange": (db, event, address) => __awaiter(void 0, void 0, void 0, function* () {
        let res = db.query("UPDATE multipool_assets SET quantity = $1 WHERE address = $2", [event.quantity, event.asset]);
        console.log(res);
    }),
    "AssetPriceChange": (db, event, address) => __awaiter(void 0, void 0, void 0, function* () {
        let res = db.query("INSERT INTO multipool_assets(address, price)\
            VALUES($2, $1)\
            ON CONFLICT(address) DO UPDATE SET\
        price = $1; ", [event.price, event.asset]);
        console.log(res);
    }),
};
