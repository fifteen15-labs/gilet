//! Rewriting a save with one member changed.
//!
//! Write support exists for exactly one job: putting a shortlist edit back
//! into the user's own save. The policy context is `LEGAL_NOTES.md` — the
//! amendment of 2 August 2026 authorises writing the user's own files, and
//! callers are expected to write a `.bak` sibling before using any of this.
//!
//! The layout being reproduced (`SAVE_FORMAT.md` §1b):
//!
//! ```text
//! 0            26-byte header; u32 at +9 = end of the member body MINUS 9
//! 26           member zstd frames, tiled back to back
//! body end     inner container `02 01 "fmf." 08 00 00`  (= u32@9 + 9)
//! body end+9   one zstd frame: the manifest, running to EOF
//! ```
//!
//! The pointer's nine-byte bias is an observed fact, not a theory: in every
//! save examined the members tile to exactly `u32@9 + 9 − 26` bytes and the
//! inner container starts right there.
//!
//! Identity is the safety proof: [`decompose`] then [`assemble`] with nothing
//! changed reproduces the input byte for byte, asserted against a real save.
//! Replacement recompresses one member, re-tiles every offset after it,
//! rewrites the manifest, and repoints the header's trailer offset.

use crate::container::{self, HEADER_LEN};
use crate::error::{Error, Result};
use crate::manifest::{self, Document};

/// Compression level for rewritten frames. FM's own level is unobservable
/// from the files; any valid zstd frame decodes, and 3 is the library default.
const LEVEL: i32 = 3;

/// A save split at its structural seams. Slices borrow the original bytes;
/// nothing is decompressed except the manifest.
#[derive(Debug)]
pub struct Decomposed<'a> {
    /// The 26-byte file header.
    pub header: &'a [u8],
    /// Every member frame, tiled — bytes 26 to the inner container.
    pub body: &'a [u8],
    /// The inner `02 01 "fmf." 08 00 00` container header.
    pub inner_header: &'a [u8],
    /// The compressed manifest frame, running to end of file.
    pub manifest_frame: &'a [u8],
    /// The decoded manifest.
    pub document: Document,
}

/// Splits a save at its structural seams and validates that the manifest
/// tiles the body exactly — the precondition for any rewrite.
///
/// # Errors
/// [`Error::Archive`] when the file's shape does not match §1b; the message
/// says which check failed. A save that fails here can still be *read* — it
/// just cannot be safely rewritten.
pub fn decompose(bytes: &[u8]) -> Result<Decomposed<'_>> {
    let header = bytes
        .get(..HEADER_LEN)
        .ok_or(Error::TooShort { len: bytes.len() })?;
    let magic = header.get(2..6).and_then(|s| <[u8; 4]>::try_from(s).ok());
    match magic {
        Some(m) if m == container::MAGIC => {}
        Some(m) => {
            return Err(Error::BadMagic {
                expected: container::MAGIC,
                found: m,
            })
        }
        None => return Err(Error::TooShort { len: bytes.len() }),
    }

    // The pointer at +9 lands nine bytes before the body actually ends —
    // see the module docs. `body_end` is where the inner container starts.
    let pointer = read_u32(header, 9).map(|v| v as usize).ok_or_else(too_short(bytes))?;
    let body_end = pointer.checked_add(9).ok_or_else(archive("body-end pointer overflows"))?;
    let body = bytes
        .get(HEADER_LEN..body_end)
        .ok_or_else(archive("body-end pointer lands outside the file"))?;
    let inner_header = bytes
        .get(body_end..body_end + 9)
        .ok_or_else(archive("inner container header is truncated"))?;
    if inner_header.get(2..6) != Some(&container::MAGIC[..]) {
        return Err(Error::Archive {
            reason: "no inner fmf. container after the body".to_owned(),
        });
    }
    let manifest_frame = bytes
        .get(body_end + 9..)
        .ok_or_else(archive("manifest frame is missing"))?;

    let (manifest_plain, consumed) =
        container::decode_one(manifest_frame).map_err(|source| Error::Archive {
            reason: format!("manifest frame does not decompress: {source}"),
        })?;
    if consumed != manifest_frame.len() {
        return Err(Error::Archive {
            reason: format!(
                "manifest frame leaves {} trailing bytes",
                manifest_frame.len() - consumed
            ),
        });
    }
    let document = manifest::parse_document(&manifest_plain)
        .ok_or_else(archive("final frame does not parse as a manifest"))?;

    // The members must tile the body exactly: any gap or overlap means this
    // file is not shaped the way the writer assumes, and writing it would
    // corrupt data the manifest does not describe.
    let mut spans: Vec<(u64, u64)> = document.members_all().map(|m| (m.offset, m.stored)).collect();
    spans.sort_unstable();
    let mut expected = 0u64;
    for (offset, stored) in spans {
        if offset != expected {
            return Err(Error::Archive {
                reason: format!("members do not tile: gap at offset {expected}"),
            });
        }
        expected = offset.saturating_add(stored);
    }
    if expected != body.len() as u64 {
        return Err(Error::Archive {
            reason: format!(
                "members cover {expected} bytes but the body is {}",
                body.len()
            ),
        });
    }

    Ok(Decomposed {
        header,
        body,
        inner_header,
        manifest_frame,
        document,
    })
}

/// Reassembles a save from its parts, repointing the header's body-end
/// pointer (with its observed nine-byte bias) at wherever the body now ends.
/// With unchanged parts this reproduces the original file byte for byte.
///
/// # Errors
/// [`Error::Archive`] if the body has grown past what a u32 pointer can hold,
/// or is shorter than the pointer's bias.
pub fn assemble(
    header: &[u8],
    body: &[u8],
    inner_header: &[u8],
    manifest_frame: &[u8],
) -> Result<Vec<u8>> {
    let pointer = (HEADER_LEN + body.len())
        .checked_sub(9)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(archive("body size does not fit the header pointer"))?;
    let mut out =
        Vec::with_capacity(HEADER_LEN + body.len() + inner_header.len() + manifest_frame.len());
    out.extend_from_slice(header);
    // Repoint in place; everything else in the header is constant across
    // every save examined and is copied through untouched.
    out.get_mut(9..13)
        .ok_or_else(archive("header shorter than 26 bytes"))?
        .copy_from_slice(&pointer.to_le_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(inner_header);
    out.extend_from_slice(manifest_frame);
    Ok(out)
}

/// The decompressed bytes of one named member, located through the manifest —
/// one frame decode instead of a whole-save parse.
///
/// # Errors
/// [`Error::Archive`] when the save's shape fails validation or no member has
/// that name; [`Error::Decompress`] when the member's frame is corrupt.
pub fn member_plaintext(bytes: &[u8], name: &str) -> Result<Vec<u8>> {
    let d = decompose(bytes)?;
    let member = d
        .document
        .members_all()
        .find(|m| m.name() == name)
        .ok_or_else(|| Error::Archive {
            reason: format!("no member named {name}"),
        })?;
    let (offset, stored) = span(member.offset, member.stored)?;
    let frame = d
        .body
        .get(offset..offset + stored)
        .ok_or_else(archive("member lies outside the body"))?;
    let (plain, _) = container::decode_one(frame).map_err(|source| Error::Archive {
        reason: format!("{name} does not decompress: {source}"),
    })?;
    if plain.len() as u64 != member.plain {
        return Err(Error::Archive {
            reason: format!(
                "{name} decompresses to {} bytes but the manifest declares {}",
                plain.len(),
                member.plain
            ),
        });
    }
    Ok(plain)
}

/// Returns the save rebuilt with `name`'s plaintext replaced. Every offset
/// after the member shifts by the size difference; the manifest and the
/// header's trailer offset follow.
///
/// # Errors
/// [`Error::Archive`] on any shape mismatch — see [`decompose`] — or an
/// unknown member name; [`Error::Decompress`] if compression itself fails.
pub fn replace_member(bytes: &[u8], name: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
    let mut d = decompose(bytes)?;

    let member = d
        .document
        .members_all()
        .find(|m| m.name() == name)
        .ok_or_else(|| Error::Archive {
            reason: format!("no member named {name}"),
        })?;
    let old_offset_u64 = member.offset;
    let (old_offset, old_stored) = span(member.offset, member.stored)?;

    let new_frame = compress(plaintext)?;

    let mut body = Vec::with_capacity(d.body.len() - old_stored + new_frame.len());
    body.extend_from_slice(d.body.get(..old_offset).ok_or_else(bad_span())?);
    body.extend_from_slice(&new_frame);
    body.extend_from_slice(d.body.get(old_offset + old_stored..).ok_or_else(bad_span())?);

    let new_stored = new_frame.len() as u64;
    let old_stored_u64 = old_stored as u64;
    for m in d.document.members_mut() {
        if m.offset == old_offset_u64 {
            m.stored = new_stored;
            m.plain = plaintext.len() as u64;
        } else if m.offset > old_offset_u64 {
            // Shift by the signed delta without leaving u64: add the new
            // size, then remove the old. Tiling was validated, so members
            // after ours sit at offsets of at least old_offset + old_stored
            // and the subtraction cannot underflow.
            m.offset = m
                .offset
                .checked_add(new_stored)
                .and_then(|o| o.checked_sub(old_stored_u64))
                .ok_or_else(archive("offset arithmetic overflowed"))?;
        }
    }

    let manifest_frame = compress(&manifest::serialize(&d.document))?;
    assemble(d.header, &body, d.inner_header, &manifest_frame)
}

fn compress(src: &[u8]) -> Result<Vec<u8>> {
    let mut ctx = zstd_safe::CCtx::create();
    let mut out = Vec::with_capacity(zstd_safe::compress_bound(src.len()));
    ctx.compress(&mut out, src, LEVEL).map_err(|code| Error::Archive {
        reason: format!("zstd compression failed: {}", zstd_safe::get_error_name(code)),
    })?;
    Ok(out)
}

/// A member's `(offset, stored)` as usize, guarding the conversion.
fn span(offset: u64, stored: u64) -> Result<(usize, usize)> {
    let offset = usize::try_from(offset).map_err(|_| Error::Archive {
        reason: "member offset exceeds the address space".to_owned(),
    })?;
    let stored = usize::try_from(stored).map_err(|_| Error::Archive {
        reason: "member length exceeds the address space".to_owned(),
    })?;
    Ok((offset, stored))
}

fn archive(reason: &str) -> impl Fn() -> Error + '_ {
    move || Error::Archive {
        reason: reason.to_owned(),
    }
}

fn bad_span() -> impl Fn() -> Error {
    || Error::Archive {
        reason: "member span lies outside the body".to_owned(),
    }
}

fn too_short(bytes: &[u8]) -> impl Fn() -> Error + '_ {
    move || Error::TooShort { len: bytes.len() }
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Builds a minimal but structurally complete save: two members, a
    /// manifest, the trailer, and a correct u32-at-9 pointer.
    fn save_with(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        let mut raws = Vec::new();
        for (name, plain) in members {
            let frame = compress(plain).unwrap();
            let (stem, ext) = name.split_at(name.find('.').unwrap());
            raws.push(crate::manifest::RawMember {
                parts: vec![stem.to_owned(), ext.to_owned()],
                offset: body.len() as u64,
                stored: frame.len() as u64,
                plain: plain.len() as u64,
                stamps: [7u8; 16],
            });
            body.extend_from_slice(&frame);
        }
        let doc = Document {
            save_name: "Test".to_owned(),
            members: raws,
            subs: Vec::new(),
            tail: vec![0u8; 4],
        };
        let manifest_frame = compress(&manifest::serialize(&doc)).unwrap();

        let mut out = Vec::new();
        out.extend_from_slice(&[0x02, 0x01]);
        out.extend_from_slice(&container::MAGIC);
        out.extend_from_slice(&[0x08, 0x00, 0x00]);
        // The pointer carries its observed nine-byte bias: body end minus 9.
        out.extend_from_slice(
            &u32::try_from(HEADER_LEN + body.len() - 9).unwrap().to_le_bytes(),
        );
        out.extend_from_slice(&[0u8; 13]); // header remainder
        out.extend_from_slice(&body);
        out.extend_from_slice(&[0x02, 0x01]);
        out.extend_from_slice(&container::MAGIC);
        out.extend_from_slice(&[0x08, 0x00, 0x00]);
        out.extend_from_slice(&manifest_frame);
        out
    }

    #[test]
    fn identity_reassembly_is_byte_identical() {
        let original = save_with(&[("a.dat", b"first member"), ("b.dat", b"second member")]);
        let d = decompose(&original).unwrap();
        let rebuilt = assemble(d.header, d.body, d.inner_header, d.manifest_frame).unwrap();
        assert_eq!(rebuilt, original);
    }

    #[test]
    fn member_plaintext_reads_by_name() {
        let save = save_with(&[("a.dat", b"first member"), ("b.dat", b"second member")]);
        assert_eq!(member_plaintext(&save, "b.dat").unwrap(), b"second member");
        assert!(member_plaintext(&save, "c.dat").is_err());
    }

    #[test]
    fn replacing_a_member_shifts_the_ones_after_it() {
        let save = save_with(&[("a.dat", b"first member"), ("b.dat", b"second member")]);
        let grown = b"a much longer first member than the original one was".as_slice();
        let rebuilt = replace_member(&save, "a.dat", grown).unwrap();

        assert_eq!(member_plaintext(&rebuilt, "a.dat").unwrap(), grown);
        assert_eq!(member_plaintext(&rebuilt, "b.dat").unwrap(), b"second member");

        // The rebuilt file still decomposes cleanly: tiling and the trailer
        // pointer were maintained, not just the two payloads.
        let d = decompose(&rebuilt).unwrap();
        assert_eq!(d.document.members.len(), 2);
    }

    #[test]
    fn a_corrupt_trailer_pointer_is_an_error_not_a_panic() {
        let mut save = save_with(&[("a.dat", b"first")]);
        let bad = u32::try_from(save.len() + 100).unwrap().to_le_bytes();
        if let Some(slot) = save.get_mut(9..13) {
            slot.copy_from_slice(&bad);
        }
        assert!(decompose(&save).is_err());
    }

    #[test]
    fn tolerates_truncation_anywhere() {
        let full = save_with(&[("a.dat", b"first member"), ("b.dat", b"second member")]);
        for cut in 0..full.len() {
            let _ = decompose(full.get(..cut).unwrap());
        }
    }
}
