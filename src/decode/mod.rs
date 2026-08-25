//! Per-message decoders, dispatched by `TransCode`. Each module documents its
//! own wire offsets and confidence level inline.

pub mod enhanced_mbp;
pub mod exec_range;
pub mod mbp;
pub mod security_range;
pub mod ticker;

use crate::message::Message;

/// Dispatches on `TransCode`. `body` starts at `BCAST_HEADER`/`TransCode`
/// (matching `hdr` in the C++ reference's decode_* functions).
pub fn dispatch(trans_code: i16, sequence: i32, body: &[u8], out: &mut Vec<Message>) {
    match trans_code {
        7208 => mbp::decode(body, sequence, out),
        17208 => enhanced_mbp::decode(body, sequence, out),
        7202 => ticker::decode(body, out),
        7305 => security_range::decode(body, out),
        7220 => exec_range::decode(body, out),
        // 7200: legacy combined MBO+MBP -- deliberately not decoded. The real
        // MBO record almost certainly carries more fields before its embedded
        // MBP array than any known usage reads, so the true offset of that
        // array isn't determinable without a fresh capture; left unimplemented
        // rather than guessed.
        7200 => {}
        _ => {}
    }
}
