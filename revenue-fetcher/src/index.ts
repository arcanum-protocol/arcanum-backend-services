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
    const response = await fetch(`https://api.llama.fi/overview/fees?excludeTotalDataChartBreakdown=true&excludeTotalDataChart=true`);
    let data = await response.json();

    const protocols = data["protocols"];
    // exclude protocols that not on arbitrum

    let results: Array<Revenue> = [];

    for (let i = 0; i < protocols.length; i++) {
        // if protocol is not on arbitrum, skip
        if (protocols[i]["chains"].indexOf("Arbitrum") === -1) {
            continue;
        }

        results.push({
            symbol: protocols[i]["displayName"],
            revenue: protocols[i]["dailyHoldersRevenue"],
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
            const revenue = Math.round(parseFloat(asset.revenue));
            if (isNaN(revenue)) {
                continue;
            }
            await pool.query(`INSERT INTO arbitrum_revenue (symbol, revenue) VALUES ($1, $2) ON CONFLICT (symbol) DO UPDATE SET revenue = $2`, [asset.symbol, revenue]);
        }
    } catch (e) {
        // console.log(e);
    }
}

// run every minute
try {
    setInterval(updateRevenueData, 1000);
} catch (e) {
    console.log(e.message);
}

