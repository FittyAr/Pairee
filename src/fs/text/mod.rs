//! Text documents for the viewer and quick view: encoding detection
//! (byte-order mark, UTF-16 heuristic, `chardetng`), decoding with
//! `encoding_rs`, and paged reading through a block cache with a line index
//! built in the background, so files of any size open at once in bounded
//! memory.

pub mod document;
pub mod encoding;
pub mod index;
pub mod lines;
pub mod search;
pub mod store;

#[cfg(test)]
mod tests;

pub use document::TextDocument;
pub use encoding::{
    DETECT_SAMPLE_BYTES, ENCODINGS, EncodingPolicy, decode_prefix, detect, encoding_by_name,
};
pub use index::IndexJob;
pub use store::{ByteStore, FileStore};
