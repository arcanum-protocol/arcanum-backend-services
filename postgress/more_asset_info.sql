ALTER TABLE assets 
ADD COLUMN volume_24h numeric not null default '0',
ADD COLUMN logo text not null default 'https://assets.coingecko.com/coins/images/1/large/bitcoin.png?1547033579',
ADD COLUMN change_24h numeric not null default '0';
