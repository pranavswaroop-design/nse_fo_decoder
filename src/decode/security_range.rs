//! 7305 -- Security (price band) update.
//! CONFIDENCE: MEDIUM. Only 3 fields are modeled; the real NSE message likely
//! carries more trailing fields (tick size, freeze qty, board lot, ...) not
//! reproduced here or in the reference. No `sequence` field in the output.

use byteorder::{BigEndian, ByteOrder};

use crate::message::{Message, SecurityRangeMessage};

pub const TOKEN_OFFSET: usize = 14; // sizeof(BCAST_HEADER)
pub const LOW_PRICE_RANGE_OFFSET: usize = 18;
pub const HIGH_PRICE_RANGE_OFFSET: usize = 22;

const TRANSACTION_CODE: i32 = 7305;
const MSG_TYPE: &str = "SECURITY_RANGE";

pub fn decode(body: &[u8], out: &mut Vec<Message>) {
    if body.len() < HIGH_PRICE_RANGE_OFFSET + 4 {
        return;
    }

    out.push(Message::SecurityRange(SecurityRangeMessage {
        transaction_code: TRANSACTION_CODE,
        msg_type: MSG_TYPE,
        token: BigEndian::read_i32(&body[TOKEN_OFFSET..]),
        low_price_range: BigEndian::read_i32(&body[LOW_PRICE_RANGE_OFFSET..]),
        high_price_range: BigEndian::read_i32(&body[HIGH_PRICE_RANGE_OFFSET..]),
    }));
}
