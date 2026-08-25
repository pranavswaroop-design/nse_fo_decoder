//! Decoded output messages. Field names/order mirror the JSON schemas produced
//! by the original C++ reference decoder exactly, so downstream consumers of
//! its output shouldn't need to change.
//!
//! `Message` is `#[serde(untagged)]` so each variant serializes as its own
//! struct's fields with no injected tag field -- `serde` preserves struct field
//! declaration order in the emitted JSON, matching the hand-built
//! `ostringstream` output in the C++ reference.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Message {
    Mbp(MbpMessage),
    Ticker(TickerMessage),
    SecurityRange(SecurityRangeMessage),
    ExecRange(ExecRangeMessage),
}

/// Shared by 7208 and 17208. For 17208, `ltp`..`total_traded_qty` are always 0
/// (real offsets UNKNOWN, see `decode::enhanced_mbp`), not genuine zero readings.
#[derive(Debug, Clone, Serialize)]
pub struct MbpMessage {
    pub transaction_code: i32,
    pub msg_type: &'static str,
    pub token: i32,
    pub sequence: i32,
    pub ltp: i32,
    pub atp: i32,
    pub close: i32,
    pub ltq: i32,
    pub ltt: i32,
    pub open: i32,
    pub high: i32,
    pub low: i32,
    pub tbq: i64,
    pub tsq: i64,
    pub total_traded_qty: i32,
    pub entries: Vec<MbpEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MbpEntry {
    pub md_entry_type: u8,
    pub level: u8,
    pub price: i32,
    pub qty: i32,
    pub orders: i16,
}

/// 7202. Note: no `sequence` field, matching the C++ reference (`decode_ticker`
/// never receives `seq`) -- confirm this is intentional before relying on it.
#[derive(Debug, Clone, Serialize)]
pub struct TickerMessage {
    pub transaction_code: i32,
    pub msg_type: &'static str,
    pub token: i32,
    pub market_type: i16,
    pub fill_price: i32,
    pub fill_volume: i32,
    pub open_interest: i32,
    pub day_high_oi: i32,
    pub day_low_oi: i32,
}

/// 7305. Only 3 fields are modeled; the real NSE message likely carries more
/// trailing fields not reproduced here.
#[derive(Debug, Clone, Serialize)]
pub struct SecurityRangeMessage {
    pub transaction_code: i32,
    pub msg_type: &'static str,
    pub token: i32,
    pub low_price_range: i32,
    pub high_price_range: i32,
}

/// 7220.
#[derive(Debug, Clone, Serialize)]
pub struct ExecRangeMessage {
    pub transaction_code: i32,
    pub msg_type: &'static str,
    pub entries: Vec<ExecRangeEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecRangeEntry {
    pub token: i32,
    pub high_exec_band: i32,
    pub low_exec_band: i32,
}
