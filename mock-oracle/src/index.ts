import { abi as ABI } from "../ABI/Multipool.json";
import { ethers, parseEther } from "ethers";
import { config } from "dotenv";
import pg from "pg";
import { BigNumber } from "@ethersproject/bignumber";

// Load environment variables from .env file
config();

// Connect to Postgres DB
const pool = new pg.Pool({
  user: process.env.DB_USER,
  password: process.env.DB_PASSWORD,
  host: process.env.DB_HOST,
  database: process.env.DB_NAME,
  port: parseInt(process.env.DB_PORT!),
});

// Connect to Ethereum wallet using private key
const provider = new ethers.JsonRpcProvider(process.env.PROVIDER_URL!);
const wallet = new ethers.Wallet(process.env.PRIVATE_KEY!, provider);

// Define contract ABI and address
const contractAddress = process.env.CONTRACT_ADDRESS!;

// Instantiate contract
const contract = new ethers.Contract(contractAddress, ABI, wallet);

// get token persent in contract
async function getTokenPersent(assetAddress: string): Promise<bigint> {
  return await contract.assetPercents(assetAddress);
}

// Define function to update price for a single token
async function updateTokenPrice(assetAddress: string, newPrice: string) {
  // Call contract function to update price
  const tx = await contract.updatePrice(assetAddress, newPrice);
  console.log(`Transaction sent: ${tx.hash}`);

  // Wait for transaction to be confirmed
  const receipt = await tx.wait();
  console.log(`Transaction confirmed in block ${receipt.blockNumber}`);
}

// Define function to get current price for a single token from Postgres
async function getTokenPrice(assetAddress: string): Promise<number> {
  const res = await pool.query(
    "SELECT price FROM assets WHERE address = $1",
    [assetAddress]
  );
  return res.rows[0].price;
}

// Define function to update prices for all tokens in the database
async function updateAllTokenPrices() {
  const addresses = await pool.query("SELECT address FROM assets");
  // exclude from addresses the address if its persent in contract is zero
  for (const row of addresses.rows) {
    try {
      const assetAddress = row.address;
      const persent = await getTokenPersent(assetAddress);
      if (persent == BigInt(0)) {
        console.log(`skipping ${row.address} because its persent is zero`);
        continue;
      }
      const newPrice = await getTokenPrice(assetAddress);
      // convert price to 18 decimal places
      const newPrice18 = parseEther(newPrice.toString()).toString();
      console.log(`updating price for ${row.address} to ${newPrice18}`);
      await updateTokenPrice(assetAddress, newPrice18);
    }
    catch (err) {
      console.log(`error updating price for ${row.address}`);
    }
  }
}

// Call function to update all token prices once per minute
setInterval(updateAllTokenPrices, 60 * 1000);
