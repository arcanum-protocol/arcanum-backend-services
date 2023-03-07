import express, { Request, Response } from 'express';
import { Pool } from 'pg';

const pool = new Pool({
  user: 'postgres',
  host: 'localhost',
  database: 'postgres',
  password: 'mysecretpassword',
  port: 5432,
});

const app = express();

// Define API endpoint to retrieve candles
app.get('/api/candles', async (req: Request, res: Response) => {
  const { timespan, countback } = req.query;

  // Validate parameters
  if (!timespan || !countback) {
    return res.status(400).json({ error: 'Missing required parameters' });
  }

  // Convert parameters to numbers
  const timespanNum = parseInt(timespan as string, 10);
  const countbackNum = parseInt(countback as string, 10);

  // Retrieve candles from database
  try {
    const result = await pool.query(
      `SELECT ts, open, close, high, low FROM candles
       WHERE ts <= $1 ORDER BY ts DESC LIMIT $2`,
      [Date.now() - timespanNum, countbackNum]
    );
    const candles = result.rows.reverse();

    // Format response in TradingView format
    const response = {
      s: 'ok',
      t: candles.map((candle) => candle.ts / 1000),
      o: candles.map((candle) => candle.open),
      c: candles.map((candle) => candle.close),
      h: candles.map((candle) => candle.high),
      l: candles.map((candle) => candle.low),
    };
    res.json(response);
  } catch (error) {
    console.error(error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

const PORT = 3000;
app.listen(PORT, () => {
  console.log(`Server listening on port ${PORT}`);
});
