// @generated automatically by Diesel CLI.

diesel::table! {
    binancepartialbook (id) {
        id -> Int4,
        #[max_length = 255]
        depth_update -> Nullable<Varchar>,
        event_timestamp -> Nullable<Int8>,
        timestamp -> Nullable<Int8>,
        #[max_length = 255]
        symbol -> Nullable<Varchar>,
        first_update_id -> Nullable<Int8>,
        final_update_id -> Nullable<Int8>,
        final_update_id_last_stream -> Nullable<Int8>,
        bids -> Nullable<Jsonb>,
        asks -> Nullable<Jsonb>,
    }
}

diesel::table! {
    binancetrades (id) {
        id -> Int4,
        #[max_length = 255]
        event_type -> Nullable<Varchar>,
        event_time -> Nullable<Int8>,
        #[max_length = 255]
        symbol -> Nullable<Varchar>,
        aggegate_id -> Nullable<Int8>,
        price -> Nullable<Float8>,
        quantity -> Nullable<Float8>,
        first_trade_id -> Nullable<Int8>,
        last_trade_id -> Nullable<Int8>,
        trade_timestamp -> Nullable<Int8>,
        is_buyer_mm -> Nullable<Bool>,
    }
}

diesel::table! {
    bookmodel (id) {
        id -> Int4,
        #[max_length = 255]
        symbol -> Nullable<Varchar>,
        timestamp -> Nullable<Int8>,
    }
}

diesel::table! {
    bookstate (id) {
        id -> Int4,
        timestamp -> Nullable<Timestamptz>,
        mid -> Nullable<Float8>,
        bb_level -> Nullable<Float8>,
        bo_level -> Nullable<Float8>,
        bb_quantity -> Nullable<Float8>,
        bo_quantity -> Nullable<Float8>,
        bid_depth -> Nullable<Float8>,
        ask_depth -> Nullable<Float8>,
        spread -> Nullable<Float8>,
        slippage_per_tick -> Nullable<Float8>,
    }
}

diesel::table! {
    quotes (id) {
        id -> Int4,
        levels -> Nullable<Float8>,
        qtys -> Nullable<Float8>,
        count -> Nullable<Int4>,
        book_model_id -> Nullable<Int4>,
    }
}

diesel::table! {
    takertrades (id) {
        id -> Int4,
        #[max_length = 255]
        symbol -> Nullable<Varchar>,
        #[max_length = 10]
        side -> Nullable<Varchar>,
        price -> Nullable<Float8>,
        qty -> Nullable<Float8>,
        local_ids -> Nullable<Int4>,
        exch_id -> Nullable<Int8>,
        transaction_timestamp -> Nullable<Int8>,
    }
}

diesel::joinable!(quotes -> bookmodel (book_model_id));

diesel::allow_tables_to_appear_in_same_query!(
    binancepartialbook,
    binancetrades,
    bookmodel,
    bookstate,
    quotes,
    takertrades,
);
