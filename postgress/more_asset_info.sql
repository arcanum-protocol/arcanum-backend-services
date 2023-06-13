--insert into assets(name, symbol, coingecko_id)
--values 
--('Bitcoin', 'BTC', 'bitcoin'),
--('Ethereum', 'ETH', 'ethereum'),
--('Ripple', 'XRP', 'ripple'),
--('Dogecoin', 'DOGE', 'dogecoin'),
--('Tron', 'TRX', 'tron'),
--('Solana', 'SOL', 'solana'),
--('Polygon', 'MATIC', 'matic-network'),
--('Shiba Inu', 'SHIB', 'shiba-inu'),
--('Uniswap', 'UNI', 'uniswap'),
--('Chainlink', 'LINK', 'link');
--
--insert into indexes(id, name, alg)
--values (default, 'Crypto X', 'mcap');
--
--insert into assets_to_indexes(index_id, asset_id)
--values
--(1, 1),
--(1, 2),
--(1, 3),
--(1, 4),
--(1, 5),
--(1, 6),
--(1, 7),
--(1, 8),
--(1, 9),
--(1, 10);

insert into assets(name, symbol, coingecko_id, defilama_id)
values 
('Gmx', 'GMX', 'gmx', '332'),
('Gains Network', 'GNS', 'gains-network', '1018'),
('Curve DEX', 'CRV', 'curve-dao-token', '3'),
('Radiant V2', 'RDNT', 'radiant-capital', '2706'),
('GND Protocol', 'GND', 'gnd-protocol', '2968'),
('SushiSwap', 'SUSHI', 'sushi', '119'),
('Camelot V2', 'GRAIL', 'camelot-token', '2307'),
('MUX protocol', 'MCB', 'mcdex', '2254'),
('Chronos', 'CHR', 'chronos-finance', '2907'),
('Ramses v1', 'RAM', 'ramses-exchange', '2675');

insert into indexes(id, name, alg, symbol)
values (default, 'Arbitrum altcoin index', 'revenue', 'ARBI');

insert into assets_to_indexes(index_id, asset_id)
values
(2, 11),
(2, 12),
(2, 13),
(2, 14),
(2, 15),
(2, 16),
(2, 17),
(2, 18),
(2, 19),
(2, 20);
