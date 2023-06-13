insert into assets(name, symbol, coingecko_id)
values 
('Bitcoin', 'BTC', 'bitcoin'),
('Ethereum', 'ETH', 'ethereum'),
('Ripple', 'XRP', 'ripple'),
('Dogecoin', 'DOGE', 'dogecoin'),
('Tron', 'TRX', 'tron'),
('Solana', 'SOL', 'solana'),
('Polygon', 'MATIC', 'matic-network'),
('Shiba Inu', 'SHIB', 'shiba-inu'),
('Uniswap', 'UNI', 'uniswap'),
('Chainlink', 'LINK', 'link');

insert into indexes(id, name, alg)
values (default, 'Crypto X', 'mcap');

insert into assets_to_indexes(index_id, asset_id)
values
(1, 1),
(1, 2),
(1, 3),
(1, 4),
(1, 5),
(1, 6),
(1, 7),
(1, 8),
(1, 9),
(1, 10);

--insert into assets(id, name, symbol, coingecko_id, defilama_id)
--values 
--(default, 'Bitcoin', 'BTC', 'bitcoin'),
--(default, 'Ethereum', 'ETH', 'ethereum'),
--(default, 'Ripple', 'XRP', 'ripple'),
--(default, 'Dogecoin', 'DOGE', 'dogecoin'),
--(default, 'Tron', 'TRX', 'tron'),
--(default, 'Solana', 'SOL', 'solana'),
--(default, 'Polygon', 'MATIC', 'matic-network'),
--(default, 'Shiba Inu', 'SHIB', 'shiba-inu'),
--(default, 'Uniswap', 'UNI', 'uniswap'),
--(default, 'Chainlink', 'LINK', 'link');
