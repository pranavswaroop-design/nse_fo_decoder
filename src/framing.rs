//! Top-level packet walk: `OUTER_PREFIX` -> `BcastPackData` -> `BcastCmpPacket`* ->
//! header -> dispatch.

use byteorder::{BigEndian, ByteOrder};

use crate::decode;
use crate::error::DecodeError;
use crate::header::{BcastHeader, HEADER_GAP, HEADER_LEN};
use crate::lzo;
use crate::message::Message;

/// Unexplained 2-byte prefix before `BcastPackData` begins on every real
/// captured datagram. Value differs per packet; meaning unknown.
pub const OUTER_PREFIX: usize = 2;

const MARKER_BYTE: u8 = 2;
/// Matches the C++ reference's fixed scratch buffer capacity. The reference
/// leaves that buffer unchecked; here `lzo::decompress_lzo1z` must respect it.
const SCRATCH_CAPACITY: usize = 2048;

/// Decodes one UDP datagram's worth of bundled/compressed NSE messages into
/// zero or more [`Message`]s. Mirrors `decode_packet` in `nse_fo_decoder.cpp`.
pub fn decode_packet(buf: &[u8]) -> Result<Vec<Message>, DecodeError> {
    let mut out = Vec::new();

    if buf.len() < OUTER_PREFIX + 2 {
        return Err(DecodeError::TooShort {
            need: OUTER_PREFIX + 2,
            got: buf.len(),
        });
    }

    let broadcast = &buf[OUTER_PREFIX..];
    let no_packets = BigEndian::read_i16(&broadcast[0..2]);
    if no_packets < 0 {
        return Ok(out);
    }

    let pack_data = &broadcast[2..];
    let buf_len = pack_data.len();
    let mut loc: usize = 0;

    for _ in 0..no_packets {
        if loc + 2 > buf_len.saturating_sub(2) {
            break;
        }

        let comp_len = BigEndian::read_i16(&pack_data[loc..loc + 2]);
        let comp_data = &pack_data[loc + 2..];

        // Body is materialized as an owned buffer (decompressed, or a copy of
        // the raw bytes) rather than borrowed in place -- simpler lifetimes
        // than the C++ reference's raw-pointer aliasing, at the cost of one
        // allocation per message. Downstream decode/* functions bound their
        // own reads explicitly, same discipline the C++ reference uses on its
        // oversized scratch-view structs.
        let (body, message_length): (Vec<u8>, i32) = if comp_len > 0 {
            let comp_len = comp_len as usize;
            if loc + 2 + comp_len > buf_len {
                break;
            }

            let mut scratch = vec![0u8; SCRATCH_CAPACITY];
            let n = lzo::decompress_lzo1z(&comp_data[..comp_len], &mut scratch)?;
            if n == 0 || scratch[0] != MARKER_BYTE {
                break;
            }
            scratch.truncate(n);
            (scratch, comp_len as i32)
        } else {
            if comp_data.is_empty() || comp_data[0] != MARKER_BYTE {
                break;
            }
            if comp_data.len() < HEADER_GAP + HEADER_LEN {
                break;
            }
            let h = match BcastHeader::parse(&comp_data[HEADER_GAP..]) {
                Some(h) => h,
                None => break,
            };
            (comp_data.to_vec(), h.message_length as i32 + HEADER_GAP as i32)
        };

        if body.len() < HEADER_GAP + HEADER_LEN {
            break;
        }
        let header = match BcastHeader::parse(&body[HEADER_GAP..]) {
            Some(h) => h,
            None => break,
        };

        decode::dispatch(header.trans_code, header.bc_seq_no, &body[HEADER_GAP..], &mut out);

        loc += message_length as usize + 2;
    }

    Ok(out)
}
