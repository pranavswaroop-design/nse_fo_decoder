//! 7202 -- Ticker & Market Index / OI.
//! CONFIDENCE: MEDIUM.
//!
//! Note: unlike `mbp`/`enhanced_mbp`, the C++ reference's `decode_ticker`
//! never receives a sequence number, and none appears in its JSON output.
//! Appends one [`Message::Ticker`] per record directly (no wrapping
//! `entries` array) -- confirm this is intentional before relying on it
//! downstream.

use byteorder::{BigEndian, ByteOrder};

use crate::message::{Message, TickerMessage};

pub const NO_OF_RECORDS_OFFSET: usize = 14; // sizeof(BCAST_HEADER)
pub const MAX_RECORDS: i16 = 40;

/// `TICKER_DATA` size, `#pragma pack(1)`: Token(4) + MktType(2) + FillPrice(4)
/// + FillVolume(4) + OI(4) + DayHiOI(4) + DayLoOI(4).
pub const RECORD_LEN: usize = 26;

const TRANSACTION_CODE: i32 = 7202;
const MSG_TYPE: &str = "OI_TICKER";

pub fn decode(body: &[u8], out: &mut Vec<Message>) {
    if body.len() < NO_OF_RECORDS_OFFSET + 2 {
        return;
    }
    let mut n = BigEndian::read_i16(&body[NO_OF_RECORDS_OFFSET..]);
    if n > MAX_RECORDS {
        n = MAX_RECORDS;
    }
    if n < 0 {
        return;
    }

    let records_start = NO_OF_RECORDS_OFFSET + 2;

    for i in 0..n as usize {
        let rec_start = records_start + i * RECORD_LEN;
        if body.len() < rec_start + RECORD_LEN {
            break;
        }
        let rec = &body[rec_start..rec_start + RECORD_LEN];

        out.push(Message::Ticker(TickerMessage {
            transaction_code: TRANSACTION_CODE,
            msg_type: MSG_TYPE,
            token: BigEndian::read_i32(&rec[0..]),
            market_type: BigEndian::read_i16(&rec[4..]),
            fill_price: BigEndian::read_i32(&rec[6..]),
            fill_volume: BigEndian::read_i32(&rec[10..]),
            open_interest: BigEndian::read_i32(&rec[14..]),
            day_high_oi: BigEndian::read_i32(&rec[18..]),
            day_low_oi: BigEndian::read_i32(&rec[22..]),
        }));
    }
}
