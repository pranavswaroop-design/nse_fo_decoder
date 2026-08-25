//! 7220 -- Trade execution range.
//! CONFIDENCE: MEDIUM.

use byteorder::{BigEndian, ByteOrder};

use crate::message::{ExecRangeEntry, ExecRangeMessage, Message};

pub const MSG_COUNT_OFFSET: usize = 14; // sizeof(BCAST_HEADER)
pub const DETAIL_OFFSET: usize = 18;
/// `TRADE_EXEC_RANGE_ENTRY` size: TokenNumber(4) + HighExecBand(4) + LowExecBand(4).
pub const ENTRY_LEN: usize = 12;
pub const MAX_ENTRIES: i32 = 80;

const TRANSACTION_CODE: i32 = 7220;
const MSG_TYPE: &str = "EXEC_RANGE";

/// Appends a single [`Message::ExecRange`] with up to `MAX_ENTRIES` entries.
pub fn decode(body: &[u8], out: &mut Vec<Message>) {
    if body.len() < MSG_COUNT_OFFSET + 4 {
        return;
    }
    let mut n = BigEndian::read_i32(&body[MSG_COUNT_OFFSET..]);
    if n > MAX_ENTRIES {
        n = MAX_ENTRIES;
    }
    if n < 0 {
        return;
    }

    let mut entries = Vec::with_capacity(n as usize);
    for i in 0..n as usize {
        let start = DETAIL_OFFSET + i * ENTRY_LEN;
        if body.len() < start + ENTRY_LEN {
            break;
        }
        let d = &body[start..start + ENTRY_LEN];
        entries.push(ExecRangeEntry {
            token: BigEndian::read_i32(&d[0..]),
            high_exec_band: BigEndian::read_i32(&d[4..]),
            low_exec_band: BigEndian::read_i32(&d[8..]),
        });
    }

    out.push(Message::ExecRange(ExecRangeMessage {
        transaction_code: TRANSACTION_CODE,
        msg_type: MSG_TYPE,
        entries,
    }));
}
