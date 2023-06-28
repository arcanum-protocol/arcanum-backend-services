// @deno-types="npm:@types/express@4.17.15"
import express, { Request, Response } from "npm:express@4.18.2";
import { Pool } from "https://deno.land/x/postgres@v0.17.0/mod.ts";

interface Candle {
  o: number;
  c: number;
  l: number;
  h: number;
  t: number;
}

(BigInt.prototype as any).toJSON = function () {
  return this.toString();
};

const DATABASE_URL = Deno.env.get("DATABASE_URL")!;
const PORT = Deno.env.get("PORT")!;

const pool = new Pool(DATABASE_URL, 10);

const app = express();

app.use((_req: Request, res: Response, next: any) => {
  res.header("Access-Control-Allow-Origin", "*");
  next();
});

app.get("/api/tv/config", (_req: Request, res: Response) => {
  res.status(200).json({
    supported_resolutions: ["1", "3", "5", "15", "30", "60", "720", "1D"],
    has_intraday: true,
    supports_group_request: false,
    supports_marks: false,
    supports_search: true,
    supports_timescale_marks: false,
  });
});

app.get("/api/tv/symbols", (req: Request, res: Response) => {
  const { symbol } = req.query;
  res.status(200).json({
    description: "Description",
    supported_resolutions: ["1", "3", "5", "15", "30", "60", "720", "1D"],
    exchange: "no",
    full_name: symbol,
    name: symbol,
    symbol: symbol,
    ticker: symbol,
    type: "Spot",
    session: "24x7",
    listed_exchange: "no",
    timezone: "Etc/UTC",
    has_intraday: true,
    minmov: 1,
    pricescale: 1000,
  });
});

// Define API endpoint to retrieve candles
app.get("/api/tv/history", async (req: Request, res: Response) => {
  const { from, to, countback, resolution, symbol } = req.query;

  const cb = Number(countback);
  const resol = resolution == "1D" ? 1440 * 60 : Number(resolution) * 60;

  console.log(`cb ${cb}, resol ${resol}, to ${to}`);
  const query = `
    SELECT
    open as o,
        close as c,
        low as l,
        high as h,
        ts as t
    FROM
    candles 
  where ts <= ${to} 
    and resolution = ${resol} 
    and index_id = (select id from indexes where symbol='${symbol}')
  ORDER BY ts DESC
  limit ${cb};
    `;

  try {
    const client = await pool.connect();
    const result = await client.queryObject(query);
    client.release();

    const rows = result.rows.reverse();
    if (rows.length == 0) {
      res.status(200).json({
        "s": "no_data",
      });
    }
    const prices = {
      "s": "ok",
      "t": rows.map((row: any) => row.t),
      "o": rows.map((row: any) => row.o),
      "c": rows.map((row: any) => row.c),
      "l": rows.map((row: any) => row.l),
      "h": rows.map((row: any) => row.h),
    };
    res.status(200).json(prices);
  } catch (err) {
    console.error(err);
    res.status(200).json({
      "s": "error",
    });
  }
});

app.get("/api/index/info", async (req: Request, res: Response) => {
  const { id } = req.query;

  const query = `
    select a.* 
    from indexes i 
        join assets_to_indexes ati 
            on ati.index_id = i.id 
        join assets a 
            on ati.asset_id = a.id
    where i.id = $1;
    `;

  const index_query = `
    select * 
    from indexes i 
    where i.id = $1;
    `;

  const client = await pool.connect();
  try {
    const assets = (await client.queryObject(query, [id])).rows;
    const index = (await client.queryObject(index_query, [id])).rows[0];

    res.status(200).json({
      "index": index,
      "assets": assets,
    });
  } catch (err) {
    console.error(err);
    res.status(500).json({
      "err": err,
    });
  }
  client.release();
});

app.get("/api/etf/info", async (req: Request, res: Response) => {
  const { address } = req.query;

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
      "assets": assets,
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
