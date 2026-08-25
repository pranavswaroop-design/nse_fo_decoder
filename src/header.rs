//! Message header parsing.
//!
//! CONFIDENCE: MEDIUM overall. `BC_SEQ_NO_OFFSET` (header + 4) contradicts what
//! the struct layout `BCAST_HEADER` in the C++ reference implies (header + 8).
//! Do not "fix" it back to 8 without re-deriving the real layout from a fresh
//! capture; header + 4 is what the reference implementation actually reads.
//!
//! The `TransCode == 0` fallback to `MESSAGE_HEADER` in the C++ reference is a
//! no-op (both structs are byte-identical layouts), so it is not ported here:
//! a single header shape is sufficient.

use byteorder::{BigEndian, ByteOrder};

/// Bytes from the marker byte (`body[0] == 2`) to `BCAST_HEADER`/`TransCode`.
/// UNEXPLAINED. The original reference assumed 8; real captures show 18.
pub const HEADER_GAP: usize = 18;

const TRANS_CODE_OFFSET: usize = 0;
/// NOT header + 8 as the "expected" struct layout implies.
const BC_SEQ_NO_OFFSET: usize = 4;
const MESSAGE_LENGTH_OFFSET: usize = 12;

/// Minimum bytes required from the header start to read every field above.
pub const HEADER_LEN: usize = MESSAGE_LENGTH_OFFSET + 2;

#[derive(Debug, Clone, Copy)]
pub struct BcastHeader {
    pub trans_code: i16,
    pub bc_seq_no: i32,
    pub message_length: i16,
}

impl BcastHeader {
    /// Parses a header starting at `buf[0]` (i.e. `buf` already points at
    /// `TransCode`, matching `hdr` in the C++ reference). Returns `None` if
    /// `buf` is too short.
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < HEADER_LEN {
            return None;
        }
        Some(Self {
            trans_code: BigEndian::read_i16(&buf[TRANS_CODE_OFFSET..]),
            bc_seq_no: BigEndian::read_i32(&buf[BC_SEQ_NO_OFFSET..]),
            message_length: BigEndian::read_i16(&buf[MESSAGE_LENGTH_OFFSET..]),
        })
    }
}
