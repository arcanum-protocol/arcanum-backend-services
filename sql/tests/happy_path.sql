--call insert_events(
--    null,
--    null,
--    1,
--    array[]::text[],
--    array[
--        '{
--            "action": "deposit",
--            "event_meta": {},
--            "block_timestamp": "2024-11-07 20:26:25",
--            "dedup_id": "3:1",
--            
--            "token_address": "0x0000000000000000000000000000000000000000",
--            "token_cryptography": "ethereum",
--            "user_address": "0x0",
--            "user_cryptography": "ethereum",
--            "amount": "1"
--        }'
--    ]::json[]
--);

DO $$ BEGIN RAISE NOTICE 'STARTING TEST'; END $$;

call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1000000, '1'::NUMERIC);
call insert_price(1, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1000000, '1'::NUMERIC);
call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1000100, '2'::NUMERIC);
call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1000800, '3'::NUMERIC);
call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1000801, '3'::NUMERIC);


DO $$ BEGIN RAISE NOTICE 'FILLING PNL'; END $$;

DO $$ BEGIN RAISE NOTICE 'INSERTING ACTIONS'; END $$;

call insert_history(
    array_to_json(array[
        '{
            "multipool":    "\\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "account":      "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "chain_id":      0,

            "action_type": "receive",

            "quantity": 100,
            "transaction_hash": "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBeaaaaaaaaaaaaaaaaaaaaaaaa",
            "timestamp": 1000000
        }'::json
    ])
);

call insert_history(
    array_to_json(array[
        '{
            "multipool":    "\\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "account":      "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "chain_id":      0,

            "action_type": "send",

            "quantity": 100,
            "transaction_hash": "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBeaaaaaaaaaaaaaaaaaaaaaaaa",
            "timestamp": 1000100
        }'::json
    ])
);

call insert_history(
    array_to_json(array[
        '{
            "multipool":    "\\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "account":      "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "chain_id":      0,

            "action_type": "receive",

            "quantity": 10,
            "transaction_hash": "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBeaaaaaaaaaaaaaaaaaaaaaaaa",
            "timestamp": 1000800
        }'::json
    ])
);

call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1004403, '10'::NUMERIC);
call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1004501, '10'::NUMERIC);

call insert_history(
    array_to_json(array[
        '{
            "multipool":    "\\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "account":      "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "chain_id":      0,

            "action_type": "send",

            "quantity": 10,
            "transaction_hash": "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBeaaaaaaaaaaaaaaaaaaaaaaaa",
            "timestamp": 1004401
        }'::json
    ])
);

call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1008001, '100'::NUMERIC);

call insert_price(0, '\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe'::ADDRESS, 1018800, '100'::NUMERIC);

call insert_history(
    array_to_json(array[
        '{
            "multipool":    "\\xc3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "account":      "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBe",
            "chain_id":      0,

            "action_type": "mint",

            "quantity": 10,
            "quote_quantity": 500,
            "transaction_hash": "\\xA3cd00388D5F1CFe441Ca43303aCaE5C22CbFDBeaaaaaaaaaaaaaaaaaaaaaaaa",
            "timestamp": 1018800
        }'::json
    ])
);


DO $$ BEGIN RAISE NOTICE 'SHOWING CANDLES'; END $$;
select * from candles;
select * from multipools;


DO $$ BEGIN RAISE NOTICE 'SHOWING POSITIONS PNL'; END $$;
select * from positions_pnl;
DO $$ BEGIN RAISE NOTICE 'SHOWING PNL'; END $$;
select * from pnl;

DO $$ BEGIN RAISE NOTICE 'SHOWING POSITIONS'; END $$;
select * from positions;
