//! 7208 -- MBP (Market by Price).
//! CONFIDENCE: HIGH (cross-checked against two independent call sites in the
//! reference implementation this crate was ported from).

use byteorder::{BigEndian, ByteOrder};

use crate::message::{Message, MbpEntry, MbpMessage};

/// `MBP_RECORD` size, `#pragma pack(1)`.
pub const RECORD_LEN: usize = 178;

// Offsets are relative to the start of one MBP_RECORD.
pub const BOOK_TYPE_OFFSET: usize = 0; // i16; must == 1 to emit this record
pub const TOKEN_OFFSET: usize = 2; // i32
pub const LEVELS_OFFSET: usize = 6; // MBP_INFO[10], 12 bytes each
pub const LEVEL_STRIDE: usize = 12; // BbBuySellFlag(2) + NumberOfOrders(2) + Price(4) + Quantity(4)
pub const LAST_TRADED_PRICE_OFFSET: usize = 126; // i32
pub const AVERAGE_TRADE_PRICE_OFFSET: usize = 130; // i32
pub const CLOSING_PRICE_OFFSET: usize = 134; // i32
pub const LAST_TRADE_QUANTITY_OFFSET: usize = 138; // i32
pub const LAST_TRADE_TIME_OFFSET: usize = 142; // i32
pub const OPEN_PRICE_OFFSET: usize = 146; // i32
pub const HIGH_PRICE_OFFSET: usize = 150; // i32
pub const LOW_PRICE_OFFSET: usize = 154; // i32
pub const TOTAL_BUY_QUANTITY_OFFSET: usize = 158; // f64
pub const TOTAL_SELL_QUANTITY_OFFSET: usize = 166; // f64
pub const VOLUME_TRADED_TODAY_OFFSET: usize = 174; // i32

/// `NoOfRecords` offset within the message body (after `BCAST_HEADER`, i.e.
/// relative to `body` in `dispatch`), clamp to 40 like the C++ reference.
pub const NO_OF_RECORDS_OFFSET: usize = 14; // sizeof(BCAST_HEADER)
pub const MAX_RECORDS: i16 = 40;

const TRANSACTION_CODE: i32 = 7208;
const MSG_TYPE: &str = "MBP";

/// Appends one [`Message::Mbp`] per record where `BookType == 1`.
pub fn decode(body: &[u8], sequence: i32, out: &mut Vec<Message>) {
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

        let book_type = BigEndian::read_i16(&rec[BOOK_TYPE_OFFSET..]);
        if book_type != 1 {
            continue;
        }

        let token = BigEndian::read_i32(&rec[TOKEN_OFFSET..]);
        let tbq = BigEndian::read_f64(&rec[TOTAL_BUY_QUANTITY_OFFSET..]) as i64;
        let tsq = BigEndian::read_f64(&rec[TOTAL_SELL_QUANTITY_OFFSET..]) as i64;

        let mut entries = Vec::with_capacity(10);
        for j in 0..10usize {
            let lvl = &rec[LEVELS_OFFSET + j * LEVEL_STRIDE..];
            let price = BigEndian::read_i32(&lvl[4..]);
            let qty = BigEndian::read_i32(&lvl[8..]);
            let orders = BigEndian::read_i16(&lvl[2..]);
            let (md_entry_type, level) = if j < 5 {
                (0u8, (j + 1) as u8)
            } else {
                (1u8, (j - 5 + 1) as u8)
            };
            entries.push(MbpEntry {
                md_entry_type,
                level,
                price,
                qty,
                orders,
            });
        }

        out.push(Message::Mbp(MbpMessage {
            transaction_code: TRANSACTION_CODE,
            msg_type: MSG_TYPE,
            token,
            sequence,
            ltp: BigEndian::read_i32(&rec[LAST_TRADED_PRICE_OFFSET..]),
            atp: BigEndian::read_i32(&rec[AVERAGE_TRADE_PRICE_OFFSET..]),
            close: BigEndian::read_i32(&rec[CLOSING_PRICE_OFFSET..]),
            ltq: BigEndian::read_i32(&rec[LAST_TRADE_QUANTITY_OFFSET..]),
            ltt: BigEndian::read_i32(&rec[LAST_TRADE_TIME_OFFSET..]),
            open: BigEndian::read_i32(&rec[OPEN_PRICE_OFFSET..]),
            high: BigEndian::read_i32(&rec[HIGH_PRICE_OFFSET..]),
            low: BigEndian::read_i32(&rec[LOW_PRICE_OFFSET..]),
            tbq,
            tsq,
            total_traded_qty: BigEndian::read_i32(&rec[VOLUME_TRADED_TODAY_OFFSET..]),
            entries,
        }));
    }
}
