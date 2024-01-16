-- Your SQL goes here

CREATE TABLE BinancePartialBook (
    id SERIAL PRIMARY KEY,
    depth_update VARCHAR(255),
    event_timestamp BIGINT,
    timestamp BIGINT,
    symbol VARCHAR(255),
    first_update_id BIGINT,
    final_update_id BIGINT,
    final_update_id_last_stream BIGINT,
    bids JSONB,
    asks JSONB
);

CREATE TABLE BinanceTrades (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(255),
    event_time BIGINT,
    symbol VARCHAR(255),
    aggegate_id BIGINT,
    price DOUBLE PRECISION,
    quantity DOUBLE PRECISION,
    first_trade_id BIGINT,
    last_trade_id BIGINT,
    trade_timestamp BIGINT,
    is_buyer_mm BOOLEAN
);

CREATE TABLE BookState (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ, -- DateTime<Utc> maps to TIMESTAMPTZ
    mid FLOAT,
    bb_level FLOAT,
    bo_level FLOAT,
    bb_quantity FLOAT,
    bo_quantity FLOAT,
    bid_depth FLOAT,
    ask_depth FLOAT,
    spread FLOAT,
    slippage_per_tick FLOAT
);

CREATE TABLE BookModel (
    id SERIAL PRIMARY KEY,
    symbol VARCHAR(255),
    timestamp BIGINT
);

CREATE TABLE Quotes (
    id SERIAL PRIMARY KEY,
    levels DOUBLE PRECISION,
    qtys DOUBLE PRECISION,
    count INTEGER,
    book_model_id INTEGER REFERENCES BookModel(id)
);
