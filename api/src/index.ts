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
    let { from, countback, resolution, index_id } = req.query;

    res.status(200).json({
        supported_resolutions: ['1', '5', '15', '30', '60', '720', '1D'],
        supports_group_request: true,
        supports_marks: false,
        supports_search: false,
        supports_timescale_marks: false,
    });
})

app.get('/api/tv/config', async (req: Request, res: Response) => {
    let { from, countback, resolution, index_id } = req.query;

    res.status(200).json({
        supported_resolutions: ['1', '5', '15', '30', '60', '720', '1D'],
        supports_group_request: true,
        supports_marks: false,
        supports_search: false,
        supports_timescale_marks: false,
    });
})

// Define API endpoint to retrieve candles
app.get('/api/tv/history', async (req: Request, res: Response) => {
    let { from, to, countback, resolution, symbol } = req.query;

    let cb = Number(countback);
    let resol = resolution == '1D' ? 1440 * 60 : Number(resolution) * 60;

    const query = `
    SELECT
    open as o,
        close as c,
        low as l,
        high as h,
        ts as t
    FROM
    candles 
  ORDER BY ts DESC
  where ts <= ${to} 
    and resolution = ${resol} 
    and index_id = (select id from indexes where symbol=${symbol})
  limit ${cb};
    `;

    try {
        const result = await pool.query(query);

        const rows = result.rows.reverse();
        if (rows.length < cb) {
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
