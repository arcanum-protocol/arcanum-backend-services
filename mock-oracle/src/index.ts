import { ethers } from "ethers";
import pg from "pg";

// Connect to Postgres DB
const pool = new pg.Pool({
  user: process.env.DB_USER,
  password: process.env.DB_PASSWORD,
  host: process.env.DB_HOST,
  database: process.env.DB_NAME,
  port: parseInt(process.env.DB_PORT),
});

// Connect to Ethereum wallet using private key
const provider = new ethers.providers.JsonRpcProvider(process.env.WEB3_PROVIDER_URL);
const wallet = new ethers.Wallet(process.env.PRIVATE_KEY, provider);

// Define contract ABI and address
const contractAbi = [...];
const contractAddress = "...";

// Instantiate contract
const contract = new ethers.Contract(contractAddress, contractAbi, wallet);

// Define function to update price for a single token
async function updateTokenPrice(assetAddress: string, newPrice: number) {
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
  const res = await pool.query("SELECT address FROM assets");
  for (const row of res.rows) {
    const assetAddress = row.address;
    const newPrice = await getTokenPrice(assetAddress);
    await updateTokenPrice(assetAddress, newPrice);
  }
}

// Call function to update all token prices once per minute
setInterval(updateAllTokenPrices, 60 * 1000);
