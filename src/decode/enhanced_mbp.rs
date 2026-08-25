//! 17208 -- Enhanced MBP.
//!
//! These are RAW WIRE OFFSETS reverse-engineered directly from captures --
//! `ENHNCD_MS_BCAST_ONLY_MBP` in the C++ header does NOT match the real
//! layout and is deliberately not used, here or in the reference. Offsets
//! below are relative to `body` (which starts at `BCAST_HEADER`/`TransCode`).
//!
//! CONFIDENCE: HIGH -- `NoOfRecords`, `Token`, the book-type flag, and each
//!   level's Quantity/Price/NumberOfOrders (16 bytes/level, 10 levels).
//! CONFIDENCE: UNKNOWN -- LTP/ATP/OHLC/TBQ/TSQ/total_traded_qty. Every sample
//!   seen so far had these as all-zero bytes; real offsets undetermined. Must
//!   be emitted as 0, not guessed, until a packet with genuine non-zero values
//!   is captured.
//! CONFIDENCE: UNVALIDATED -- the record-to-record stride for `NoOfRecords >
//!   1`. Only `NoOfRecords == 1` has been observed; decode only the first
//!   record and warn (to stderr) if more are claimed, same as the reference.

use byteorder::{BigEndian, ByteOrder};

use crate::message::{Message, MbpEntry, MbpMessage};

pub const NO_OF_RECORDS_OFFSET: usize = 30;
pub const TOKEN_OFFSET: usize = 32;
pub const BOOK_TYPE_OFFSET: usize = 36;
pub const LEVELS_OFFSET: usize = 96;
/// Quantity(4) + Price(4) + NumberOfOrders(2) + 6 unknown/padding bytes.
pub const LEVEL_STRIDE: usize = 16;
pub const MAX_RECORDS: i16 = 40;

const TRANSACTION_CODE: i32 = 17208;
const MSG_TYPE: &str = "MBP";

/// Appends at most one [`Message::Mbp`] (transaction_code 17208) if
/// `NoOfRecords >= 1` and the book-type flag == 1. Additional records beyond
/// the first are unvalidated and must not be decoded -- warn instead.
pub fn decode(body: &[u8], sequence: i32, out: &mut Vec<Message>) {
    if body.len() < NO_OF_RECORDS_OFFSET + 2 {
        return;
    }
    let mut n = BigEndian::read_i16(&body[NO_OF_RECORDS_OFFSET..]);
    if n > MAX_RECORDS {
        n = MAX_RECORDS;
    }
    if n < 0 {
        n = 0;
    }
    if n > 1 {
        eprintln!(
            "[nse_fo_decoder] warning: 17208 message has NoOfRecords={n} but the \
             multi-record stride is unvalidated -- decoding only the first record."
        );
        n = 1;
    }

    if body.len() < LEVELS_OFFSET + 10 * LEVEL_STRIDE {
        return;
    }

    for _ in 0..n {
        let book_type = BigEndian::read_i16(&body[BOOK_TYPE_OFFSET..]);
        if book_type != 1 {
            continue;
        }

        let token = BigEndian::read_i32(&body[TOKEN_OFFSET..]);

        let mut entries = Vec::with_capacity(10);
        for j in 0..10usize {
            let lvl = &body[LEVELS_OFFSET + j * LEVEL_STRIDE..];
            let qty = BigEndian::read_i32(&lvl[0..]);
            let price = BigEndian::read_i32(&lvl[4..]);
            let orders = BigEndian::read_i16(&lvl[8..]);
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
            ltp: 0,
            atp: 0,
            close: 0,
            ltq: 0,
            ltt: 0,
            open: 0,
            high: 0,
            low: 0,
            tbq: 0,
            tsq: 0,
            total_traded_qty: 0,
            entries,
        }));
    }
}
