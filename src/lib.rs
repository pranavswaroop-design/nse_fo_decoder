//! NSE F&O broadcast (NNF) decoder.
//!
//! This is a reverse-engineered wire format: large parts of the layout are
//! confirmed only against a small number of captured packets. Confidence
//! levels (HIGH / MEDIUM / UNKNOWN / UNVALIDATED) noted throughout the doc
//! comments in this crate are load-bearing -- don't silently upgrade them
//! without new evidence from a real captured packet.

pub mod decode;
pub mod error;
pub mod framing;
pub mod header;
pub mod lzo;
pub mod message;

pub use error::DecodeError;
pub use framing::decode_packet;
pub use message::Message;
