// https://tokenterminal.com/_next/data/svrWqEDxQjHKkOInOzf2_/leaderboards/earnings.json

import pg from 'pg';
import { config } from "dotenv";
import { BigNumber } from 'bignumber.js';
import express, { Request, Response } from 'express';

// Load environment variables from .env file
config();

const pool = new pg.Pool({
    user: process.env.DB_USER,
    password: process.env.DB_PASSWORD,
    host: process.env.DB_HOST,
    database: process.env.DB_NAME,
    port: parseInt(process.env.DB_PORT!),
});

interface Revenue {
    symbol: string;
    revenue: string;
}

async function getRevenueData(): Promise<Array<Revenue>> {
    const response = await fetch(`https://tokenterminal.com/_next/data/svrWqEDxQjHKkOInOzf2_/leaderboards/earnings.json`);
    console.log("response", response);
    let data = await response.json();
    let results: Array<Revenue> = [];

    const cleanData = data["pageProps"]["earningsData"];
    
    for (let i = 0; i < cleanData.length; i++) {
        results.push({
            symbol: cleanData[i]["projectId"],
            revenue: cleanData[i]["revenue"]["1d"],
        });
    }
    return results;
}

async function updateRevenueData(): Promise<void> {
    // const client = await pool.connect();
    let revenueData = await getRevenueData();
    // update assets table
    try {
        for (let i = 0; i < revenueData.length; i++) {
            const asset = revenueData[i];
            await pool.query('UPDATE assets SET revenue = $1 WHERE coingecko_id = $3',
                [asset.revenue, asset.symbol]);
        }
    } catch (e) {
        console.log(e);
    }
}

// run every minute
try {
    setInterval(updateRevenueData, 1000);
} catch (e) {
    console.log(e.message);
}

