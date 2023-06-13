import express, { Request, Response } from 'express';
import { Pool } from 'pg';

interface Candle {
    o: number;
    c: number;
    l: number;
    h: number;
    t: number;
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



app.get('/api/tv/config', async (req: Request, res: Response) => {
    res.status(200).json({
        supported_resolutions: ['1', '3', '5', '15', '30', '60', '720', '1D'],
        has_intraday: true,
        supports_group_request: false,
        supports_marks: false,
        supports_search: true,
        supports_timescale_marks: false,
    });
})

app.get('/api/tv/symbols', async (req: Request, res: Response) => {
    let { symbol } = req.query;
    res.status(200).json({
        description: 'Description',
        supported_resolutions: ['1', '3', '5', '15', '30', '60', '720', '1D'],
        exchange: 'no',
        full_name: symbol,
        name: symbol,
        symbol: symbol,
        ticker: symbol,
        type: 'Spot',
        session: '24x7',
        listed_exchange: 'no',
        timezone: 'Etc/UTC',
        has_intraday: true,
        minmov: 1,
        pricescale: 1000,
    });
})

// Define API endpoint to retrieve candles
app.get('/api/tv/history', async (req: Request, res: Response) => {
    let { from, to, countback, resolution, symbol } = req.query;

    let cb = Number(countback);
    let resol = resolution == '1D' ? 1440 * 60 : Number(resolution) * 60;

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
        const result = await pool.query(query);

        const rows = result.rows.reverse();
        if (rows.length == 0) {
            res.status(200).json({
                "s": "no_data",
            });
        }
        const prices = {
            "s": "ok",
            "t": rows.map((row: Candle) => row.t),
            "o": rows.map((row: Candle) => row.o),
            "c": rows.map((row: Candle) => row.c),
            "l": rows.map((row: Candle) => row.l),
            "h": rows.map((row: Candle) => row.h),
        }
        res.status(200).json(prices);
    } catch (err) {
        console.error(err);
        res.status(200).json({
            "s": "error",
        });
    }

});

const PORT = process.env.PORT;
app.listen(PORT, () => {
    console.log(`Server listening on port ${PORT} `);
});
