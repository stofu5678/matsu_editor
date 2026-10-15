//! The codec the buffer uses: [`crate::encoding`] (the contract with the encoding module).
//!
//! The document needs an `Arc<dyn Codec>`, while [`Encoding::codec`] gives a `&'static dyn Codec`.
//! [`EncodingCodec`] is the small adapter between them.

pub use crate::encoding::{Codec, Eol, INVALID_BYTE_BASE, Unmappable};

use crate::encoding::Encoding;

/// An [`Encoding`] as an owned [`Codec`].
#[derive(Clone, Copy, Debug)]
pub struct EncodingCodec(pub Encoding);

impl Codec for EncodingCodec {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        self.0.codec().decode(bytes, out);
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        self.0.codec().find_eol(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        self.0.codec().eol_bytes(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        self.0.codec().char_len(bytes, i)
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        self.0.codec().encode(text, out)
    }

    fn is_stateless(&self) -> bool {
        self.0.codec().is_stateless()
    }
}

/// UTF-8, which the add buffer always uses (typed text is UTF-8 whatever the file's encoding is).
pub(crate) fn utf8() -> &'static dyn Codec {
    Encoding::Utf8.codec()
}
