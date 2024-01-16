
CREATE TABLE TakerTrades (
    symbol VARCHAR(255),
    side VARCHAR(10), 
    price DOUBLE PRECISION,
    qty DOUBLE PRECISION,
    local_ids INTEGER,
    exch_id BIGINT,
    transaction_timestamp BIGINT
);
