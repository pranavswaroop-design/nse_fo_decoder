use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("input too short: need at least {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },

    #[error("marker byte mismatch: expected 2")]
    BadMarker,

    #[error("LZO1Z decompression failed: {0}")]
    Lzo(#[from] crate::lzo::LzoError),
}
