import express, { Request, Response } from 'express';
import { Pool } from 'pg';

interface Candle {
  ts: number;
  price: number;
}

const pool = new Pool({
  connectionString: process.env.DATABASE_URL,
});

const app = express();

//app.use(cors());
app.use((_req, res, next) => {
  res.header('Access-Control-Allow-Origin', '*');
  next();
});


// Define API endpoint to retrieve candles
app.get('/api/prices', async (req: Request, res: Response) => {
  let { limit } = req.query;

  const query = `
  SELECT 
    * 
  FROM 
    prices 
  ORDER BY ts DESC
  limit ${limit};
  `;

  try {
    const result = await pool.query(query);

    const rows = result.rows.reverse();
    const prices = {
      "status": "ok",
      "t": rows.map((row: Candle) => row.ts),
      "p": rows.map((row: Candle) => row.price),
    }
    res.status(200).json(prices);
  } catch (err) {
    console.error(err);
    res.status(500).send('Internal Server Error');
  }

});

app.get('/api/assets', async (_req: Request, res: Response) => {
  const query = `
  SELECT
    *
  FROM
    assets
  `;
  try {
    const result = await pool.query(query);
    res.status(200).json(result.rows);
  } catch (err) {
    console.error(err);
    res.status(500).send('Internal Server Error');
  }
});

app.get('/api/multipool_assets', async (_req: Request, res: Response) => {
  const query = `
    select * 
    from multipool_assets m 
      left join mp_to_asset j on j.mp_address=m.address 
      left join assets a on a.coingecko_id=j.asset_id
  `;
  try {
    const result = await pool.query(query);
    res.status(200).json(result.rows);
  } catch (err) {
    console.error(err);
    res.status(500).send('Internal Server Error');
  }
});

const PORT = process.env.PORT;
app.listen(PORT, () => {
  console.log(`Server listening on port ${PORT}`);
});
