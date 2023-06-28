// @deno-types="npm:@types/express@4.17.15"
import express, { Request, Response } from "npm:express@4.18.2";
import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";
import Web3 from "npm:web3";
import { ABI } from "./multipool-router-abi.ts";

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

app.use((_req: Request, res: Response, next: any) => {
  res.header("Access-Control-Allow-Origin", "*");
  next();
});

app.get("/swap/v1/quote", async (req: Request, res: Response) => {
  const {
    buyToken,
    sellToken,
    sellAmount,
    buyAmount,
    slippagePercentage,
  } = req.query;

  let result = {
    price: 0,
    grossPrice: 0,
    guaranteedPrice: 0,
    to: CONTRACT_ADDRESS,
    data: [],
    buyAmount: 0,
    grossBuyAmount: 0,
    sellAmount: 0,
    grossSellAmount: 0,
    buyTokenAddress: "",
    sellTokenAddress: "",
    allowanceTarget: "",
    sellTokenToEthRate: "",
    buyTokenToEthRate: "",
    expectedSlippage: 0,
  };

  const query = `
    select * 
    from etf_assets ea
        join assets a 
            on a.id = ea.asset_id 
    where ea.multipool_address = $1;
    `;

  const client = await pool.connect();
  try {
    const assets: any =
      (await client.queryObject(query, [address.toLowerCase()])).rows;

    res.status(200).json({
      "assets": parsed_assets,
    });
  } catch (err) {
    console.error(err);
    res.status(500).json({
      "err": err,
    });
  }
  client.release();
});

app.listen(PORT, () => {
  console.log(`Server listening on port ${PORT}`);
});
