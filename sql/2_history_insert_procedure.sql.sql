CREATE OR REPLACE PROCEDURE insert_history(changes JSON)
   LANGUAGE PLPGSQL
AS
$$
BEGIN
    --TODO: cleanup temp tables

        -- Parse new trading history records from JSON into temp table
        CREATE TEMP TABLE temp_trading_history (
            LIKE trading_history INCLUDING ALL
        ) ON COMMIT DROP;
        ALTER TABLE temp_trading_history ALTER COLUMN quote_quantity DROP NOT NULL;

        
        INSERT INTO temp_trading_history
        SELECT * FROM json_populate_recordset(null::temp_trading_history, changes);


        -- Fill quote_quantity in the moment of send/receive if this suitable action
        UPDATE temp_trading_history 
        SET quote_quantity = quantity * (
            SELECT close 
            FROM candles 
            -- NOTICE: minute candle for every timestamp should exist
            WHERE candles.multipool = multipool and candles.chain_id = chain_id and resolution = 60 and candles.ts = temp_trading_history.timestamp / 60 * 60
            LIMIT 1
        ) WHERE action_type = 'send' OR action_type = 'receive';

        ALTER TABLE temp_trading_history ALTER COLUMN quote_quantity SET NOT NULL;

        -- With having a lock updating all positions one by another
        DECLARE
            change trading_history;
        BEGIN
            FOR change IN SELECT * FROM temp_trading_history LOOP

                -- Update position pnl records with balance change (applied to all changes after action)
                DECLARE
                    var_quantity_delta NUMERIC := 
                            CASE WHEN change.action_type = 'mint' OR change.action_type = 'receive' THEN  change.quantity   
                                WHEN change.action_type = 'burn' OR change.action_type = 'send' THEN - change.quantity  END;
                BEGIN

                IF NOT EXISTS (
                    SELECT * FROM positions_pnl
                    WHERE 
                        timestamp = change.timestamp / 3600 * 3600 AND 
                        chain_id = change.chain_id AND 
                        account = change.account AND 
                        multipool = change.multipool
                ) THEN
                    DECLARE 
                        var_last_pnl_record positions_pnl := (SELECT positions_pnl FROM positions_pnl WHERE chain_id = change.chain_id AND account = change.account AND multipool = change.multipool ORDER BY timestamp DESC LIMIT 1);
                        var_acc_profit NUMERIC := coalesce(var_last_pnl_record.acc_profit, 0);
                        var_acc_loss NUMERIC := coalesce(var_last_pnl_record.acc_loss, 0);
                        var_current_candle candles;
                    BEGIN
                        FOR var_current_candle IN 
                            SELECT * FROM candles 
                            WHERE chain_id = change.chain_id AND multipool = change.multipool AND resolution = 3600 AND ts >= change.timestamp / 3600 * 3600
                        LOOP
                            INSERT INTO positions_pnl(chain_id, account, multipool, timestamp, acc_profit, acc_loss, open_quantity, close_quantity, open_price, close_price)
                            VALUES (
                                change.chain_id, 
                                change.account, 
                                change.multipool, 
                                var_current_candle.ts, 
                                var_acc_profit,
                                var_acc_loss, 
                                0,
                                0,
                                var_current_candle.open, 
                                var_current_candle.close
                            ) ON CONFLICT (chain_id, account, multipool, timestamp) DO NOTHING;
                        END LOOP;
                    END;
                END IF;

                UPDATE positions_pnl SET 
                        acc_profit = acc_profit + CASE WHEN change.action_type = 'burn' OR change.action_type = 'send' THEN change.quote_quantity ELSE 0 END,
                        acc_loss = acc_loss + CASE WHEN change.action_type = 'mint' OR change.action_type = 'receive' THEN change.quote_quantity ELSE 0 END,
                        open_quantity = open_quantity + CASE WHEN timestamp != change.timestamp / 3600 * 3600 THEN var_quantity_delta ELSE 0 END,
                        close_quantity = close_quantity + var_quantity_delta
                    WHERE
                        chain_id = change.chain_id 
                        AND account = change.account 
                        AND multipool = change.multipool 
                        AND timestamp >= change.timestamp / 3600 * 3600;

                    -- Update positions table
                    DECLARE
                        var_current_position positions := (SELECT positions FROM positions WHERE positions.chain_id = change.chain_id AND positions.account = change.account AND positions.multipool = change.multipool);
                    BEGIN
                        IF var_current_position IS NULL THEN
                            INSERT INTO positions(chain_id, account, multipool, quantity, opened_at) 
                            VALUES (change.chain_id, change.account, change.multipool, var_quantity_delta, change.timestamp);
                        ELSIF var_current_position.quantity + var_quantity_delta = 0 THEN
                            DELETE FROM positions
                            WHERE chain_id = change.chain_id AND account = change.account AND multipool = change.multipool;
                        ELSE
                            UPDATE positions SET quantity = quantity + var_quantity_delta 
                            WHERE chain_id = change.chain_id AND account = change.account AND multipool = change.multipool;
                        END IF;
                    END;
                END;
            END LOOP;
        END;
END
$$;

CREATE OR REPLACE FUNCTION update_total_pnl()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO pnl(chain_id, account, timestamp, acc_profit, acc_loss, open_quote, close_quote) 
        VALUES (NEW.chain_id, NEW.account, NEW.timestamp, NEW.acc_profit, NEW.acc_loss, (NEW.open_price * NEW.open_quantity), (NEW.close_price * NEW.close_quantity)) 
        ON CONFLICT (chain_id, account, timestamp) DO UPDATE SET
            acc_profit = pnl.acc_profit + NEW.acc_profit,
            acc_loss = pnl.acc_loss + NEW.acc_loss,
            open_quote = pnl.open_quote + (NEW.open_price * NEW.open_quantity),
            close_quote = pnl.close_quote + (NEW.close_price * NEW.close_quantity);
        RETURN NEW; 
    ELSIF TG_OP = 'UPDATE' THEN
        UPDATE pnl SET
            acc_profit = acc_profit - OLD.acc_profit + NEW.acc_profit,
            acc_loss = acc_loss - OLD.acc_loss + NEW.acc_loss,
            open_quote = open_quote - (OLD.open_price * OLD.open_quantity) + (NEW.open_price * NEW.open_quantity),
            close_quote = close_quote - (OLD.close_price * OLD.close_quantity) + (NEW.close_price * NEW.close_quantity)
        WHERE pnl.account = NEW.account AND pnl.chain_id = NEW.chain_id AND pnl.timestamp = NEW.timestamp;
        RETURN NEW; 
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE pnl SET
            acc_profit = acc_profit - OLD.acc_profit,
            acc_loss = acc_loss - OLD.acc_loss,
            open_quote = open_quote - (OLD.open_price * OLD.open_quantity),
            close_quote = close_quote - (OLD.close_price * OLD.close_quantity)
        WHERE pnl.account = OLD.account AND pnl.chain_id = OLD.chain_id AND pnl.timestamp = OLD.timestamp;
        -- Maybe also delete?
        RETURN OLD; 
    END IF;
    RETURN NULL; 
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_total_pnl
AFTER INSERT OR UPDATE OR DELETE ON positions_pnl
FOR EACH ROW EXECUTE FUNCTION update_total_pnl();

