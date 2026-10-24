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

    fn boundary_at_or_after(&self, bytes: &[u8], from: usize, target: usize) -> usize {
        self.0.codec().boundary_at_or_after(bytes, from, target)
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

/// Finds the next line ending of `bytes` from `from`. NEL, LS and PS count only when `ext_eol` is
/// set (`WCODE::IsLineDelimiter(wc, bEnableExtEol)`); otherwise they are skipped as ordinary
/// characters. With `ext_eol` set this is `codec.find_eol`.
pub(crate) fn find_eol(
    codec: &dyn Codec,
    bytes: &[u8],
    mut from: usize,
    ext_eol: bool,
) -> Option<(usize, Eol)> {
    loop {
        let (off, kind) = codec.find_eol(bytes, from)?;
        if ext_eol || matches!(kind, Eol::CrLf | Eol::Lf | Eol::Cr) {
            return Some((off, kind));
        }
        let len = codec.eol_bytes(kind).map_or(0, <[u8]>::len);
        // A misbehaving codec must not make this loop forever.
        if off < from || len == 0 {
            return None;
        }
        from = off + len;
    }
}
