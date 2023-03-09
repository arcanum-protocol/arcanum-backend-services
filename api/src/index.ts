import express, { Request, Response } from 'express';
import { Pool } from 'pg';
import cors from 'cors';

interface Candle {
  ts_group: number;
  open: number;
  close: number;
  high: number;
  low: number;
}

enum Timespan {
  "1m" = 60,
  "5m" = 5 * 60,
  "15m" = 15 * 60,
  "30m" = 30 * 60,
  "1h" = 60 * 60,
  "4h" = 4 * 60 * 60,
  "1d" = 24 * 60 * 60,
  "1w" = 7 * 24 * 60 * 60,
  "1M" = 30 * 24 * 60 * 60,
}

const pool = new Pool({
  user: 'postgres',
  host: 'localhost',
  database: 'postgres',
  password: 'mysecretpassword',
  port: 5432,
});

const app = express();

const allowedOrigins = ['http://localhost:9999'];
app.use(cors({
  origin: function(origin, callback){
    if(!origin) return callback(null, true);
    if(allowedOrigins.indexOf(origin) === -1){
      const msg = 'The CORS policy for this site does not allow access from the specified Origin.';
      return callback(new Error(msg), false);
    }
    return callback(null, true);
  }

}));

// Define API endpoint to retrieve candles
app.get('/api/candles', async (req: Request, res: Response) => {
  let { countback, timespan } = req.query;

  const pgCountback = Number(countback);
  const pgTimespan = Timespan[timespan as keyof typeof Timespan];

  const now = Math.floor(Date.now() / 1000);
  const startTime = (now - pgTimespan * pgCountback);
  const endTime = now;

  console.log(`startTime: ${startTime}, endTime: ${endTime}, timespan: ${pgTimespan}`);

  const query = `
  SELECT 
    FLOOR(ts / ${pgTimespan}) * ${pgTimespan} AS ts_group,
    MAX(high) AS high,
    MIN(low) AS low,
    (SELECT open FROM candles c2 WHERE c2.ts = MIN(c1.ts)) AS open,
    (SELECT close FROM candles c3 WHERE c3.ts = MAX(c1.ts)) AS close
  FROM 
    candles c1
  WHERE c1.ts > ${startTime} AND c1.ts < ${endTime}
  GROUP BY 
    ts_group
  ORDER BY 
    ts_group ASC;
  `;

  try {
    const result = await pool.query(query);

    const candles = {
      "status": "ok",
      "ts": result.rows.map((row: Candle) => row.ts_group),
      "open": result.rows.map((row: Candle) => row.open),
      "close": result.rows.map((row: Candle) => row.close),
      "high": result.rows.map((row: Candle) => row.high),
      "low": result.rows.map((row: Candle) => row.low),
    }
    res.status(200).json(candles);
  } catch (err) {
    console.error(err);
    res.status(500).send('Internal Server Error');
  }

});

const PORT = 3000;
app.listen(PORT, () => {
  console.log(`Server listening on port ${PORT}`);
});
