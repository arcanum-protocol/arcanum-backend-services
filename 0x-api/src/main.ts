// @deno-types="npm:@types/express@4.17.15"
import express, { Request, Response } from "npm:express@4.18.2";
import morgan from "npm:morgan@1.10.0";
import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import Web3 from "npm:web3";
import { ABI } from "./multipool-router-abi.ts";

(BigInt.prototype as any).toJSON = function () {
  return this.toString();
};

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const PORT = Deno.env.get("PORT")!;
const CONTRACT_ADDRESS = Deno.env.get("CONTRACT_ADDRESS")!.toLowerCase();
const ROUTER_CONTRACT_ADDRESS = Deno.env.get("ROUTER_CONTRACT_ADDRESS")!
  .toLowerCase();
const PROVIDER_URL = Deno.env.get("PROVIDER_URL")!;

const web3 = new Web3(PROVIDER_URL);
const contract = new web3.eth.Contract(ABI, ROUTER_CONTRACT_ADDRESS);

const pool = new Pool(DATABASE_URL, 10);

const app = express();

app.use(morgan("combined"));

app.use((_req: Request, res: Response, next: any) => {
  res.header("Access-Control-Allow-Origin", "*");
  next();
});

app.get("/swap/v1/quote", async (req: Request, res: Response) => {
  try {
    const {
      buyToken,
      sellToken,
      sellAmount,
      buyAmount,
      slippagePercentage,
    } = req.query;

    let returnValue = {
      to: CONTRACT_ADDRESS,
      data: [],
      buyAmount: "",
      sellAmount: "",
      buyTokenAddress: buyAmount,
      sellTokenAddress: sellToken,
      allowanceTarget: sellToken,
      expectedSlippage: slippagePercentage,
    };

    if (sellAmount) {
      let result = await contract.methods.estimateSwapSharesByAmountIn(
        CONTRACT_ADDRESS,
        sellToken,
        buyToken,
        sellAmount,
      ).call();
      returnValue.buyAmount = result.amountOut;
      returnValue.sellAmount = String(sellAmount);
      let transaction = await contract.methods.swapWithAmountIn(
        CONTRACT_ADDRESS,
        sellToken,
        buyToken,
        sellAmount,
        result.amountOut *
          BigInt(
            (1 - Number(slippagePercentage || "0")) * Math.pow(10, 18),
          ) /
          BigInt(Math.pow(10, 18)),
        "0x0000000000000000000000000000000000000000",
        1908978745,
      );
      returnValue.data = transaction.encodeABI().toString();
    } else if (buyAmount) {
      let result = await contract.methods.estimateSwapSharesByAmountOut(
        CONTRACT_ADDRESS,
        sellToken,
        buyToken,
        buyAmount,
      ).call();
      returnValue.sellAmount = result.amountIn;
      returnValue.buyAmount = String(buyAmount);
      let transaction = await contract.methods.swapWithAmountOut(
        CONTRACT_ADDRESS,
        sellToken,
        buyToken,
        buyAmount,
        result.amountIn *
          BigInt(
            (1 - Number(slippagePercentage || "0")) * Math.pow(10, 18),
          ) /
          BigInt(Math.pow(10, 18)),
        "0x0000000000000000000000000000000000000000",
        1908978745,
      );
      returnValue.data = transaction.encodeABI().toString();
    }

    res.status(200).json(returnValue);
  } catch {
    res.status(500).send();
  }
});

app.listen(PORT, () => {
  console.log(`Server listening on port ${PORT}`);
});
