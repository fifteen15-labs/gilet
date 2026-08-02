//! The save's member manifest: the last frame names every other frame.
//!
//! A save is an archive, not an anonymous frame stream. Its final zstd frame —
//! the only one without the `03 01 "tad."` prefix — is a manifest in the same
//! member layout as the `.fmf` shortlist archive (`SHORTLIST_FORMAT.md` §3):
//!
//! ```text
//! u32 len + bytes     save name
//! u32                 top-level member count
//! per member:
//!   u32 len + bytes   name parts, repeated until one begins with '.'
//!   u64               offset, relative to the 26-byte file header
//!   u64               stored (compressed) length
//!   u64               plaintext (decompressed) length
//!   16 bytes          two stamps
//! u32                 sub-archive count
//! per sub-archive:    u32 len + name, u32 child count, children as above
//! ```
//!
//! Members sorted by offset are exactly the frames in file order, so the
//! sorted position of a member is its frame index. Verified in
//! `SAVE_FORMAT.md` §1b: all 1,214 members of the reference save match their
//! frames byte for byte.

/// One named member of the save archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// File name, e.g. `game_db.dat`; sub-archive children are prefixed with
    /// the archive name, e.g. `rgman/rule_group.dat`.
    pub name: String,
    /// Offset of the member's zstd frame, relative to the file header.
    pub offset: u64,
    /// Compressed length on disk.
    pub stored: u64,
    /// Decompressed length, used to cross-check a member against its frame.
    pub plain: u64,
}

/// A member as the manifest stores it, kept in full so the manifest can be
/// written back byte for byte. The write path (`archive.rs`) edits lengths
/// and offsets and must not disturb anything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMember {
    /// Name parts exactly as stored — the last begins with `'.'`.
    pub parts: Vec<String>,
    pub offset: u64,
    pub stored: u64,
    pub plain: u64,
    /// Two 8-byte stamps, values consistent with unix timestamps. Opaque here.
    pub stamps: [u8; 16],
}

impl RawMember {
    /// The joined file name, e.g. `game_db.dat`.
    #[must_use]
    pub fn name(&self) -> String {
        self.parts.concat()
    }
}

/// The manifest with its structure intact: top-level members, then named
/// sub-archives, then whatever trailing bytes follow (four zero bytes in
/// every save examined). [`serialize`] reproduces the source exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub save_name: String,
    pub members: Vec<RawMember>,
    pub subs: Vec<(String, Vec<RawMember>)>,
    pub tail: Vec<u8>,
}

impl Document {
    /// Every member across all sections, mutably — the write path shifts
    /// offsets without caring which section a member sits in.
    pub fn members_mut(&mut self) -> impl Iterator<Item = &mut RawMember> {
        self.members
            .iter_mut()
            .chain(self.subs.iter_mut().flat_map(|(_, m)| m.iter_mut()))
    }

    /// Every member across all sections.
    pub fn members_all(&self) -> impl Iterator<Item = &RawMember> {
        self.members
            .iter()
            .chain(self.subs.iter().flat_map(|(_, m)| m.iter()))
    }
}

/// A member or archive name never legitimately reaches this length; a longer
/// read is the walk losing sync, not a long name.
const MAX_NAME: usize = 256;

/// Name parts per member: every observed member is `stem + ".ext"`, but the
/// format allows more, so allow a few before calling the walk lost.
const MAX_NAME_PARTS: usize = 8;

/// Reads the manifest from the decompressed final frame, returning members
/// sorted by offset — index in the result is frame index. `None` when the
/// frame does not parse as a manifest, which callers treat as "this save does
/// not name its members" rather than an error.
#[must_use]
pub fn read_manifest(frame: &[u8]) -> Option<Vec<Member>> {
    let doc = parse_document(frame)?;
    let mut members: Vec<Member> = doc
        .members
        .iter()
        .map(|m| (None, m))
        .chain(
            doc.subs
                .iter()
                .flat_map(|(sub, ms)| ms.iter().map(move |m| (Some(sub.as_str()), m))),
        )
        .map(|(sub, m)| Member {
            name: match sub {
                Some(sub) => format!("{sub}/{}", m.name()),
                None => m.name(),
            },
            offset: m.offset,
            stored: m.stored,
            plain: m.plain,
        })
        .collect();
    members.sort_by_key(|m| m.offset);
    Some(members)
}

/// The frame index of the named member, resolved through the sorted manifest.
#[must_use]
pub fn frame_index_of(members: &[Member], name: &str) -> Option<usize> {
    members.iter().position(|m| m.name == name)
}

/// Parses the manifest keeping every byte of structure, so it can be edited
/// and written back. `None` when the frame is not a manifest.
#[must_use]
pub fn parse_document(frame: &[u8]) -> Option<Document> {
    let mut pos = 0usize;
    let (save_name, next) = read_string(frame, pos)?;
    pos = next;

    let count = read_u32(frame, pos)? as usize;
    pos += 4;
    let (members, next) = read_raw_members(frame, pos, count)?;
    pos = next;

    let sub_count = read_u32(frame, pos)? as usize;
    pos += 4;
    let mut subs = Vec::new();
    for _ in 0..sub_count {
        let (sub_name, next) = read_string(frame, pos)?;
        pos = next;
        let children = read_u32(frame, pos)? as usize;
        pos += 4;
        let (childs, next) = read_raw_members(frame, pos, children)?;
        pos = next;
        subs.push((sub_name, childs));
    }

    let tail = frame.get(pos..)?.to_vec();
    Some(Document {
        save_name,
        members,
        subs,
        tail,
    })
}

/// Writes a [`Document`] back to bytes. Parsing and serialising an untouched
/// manifest reproduces the original exactly — asserted against a real save.
#[must_use]
pub fn serialize(doc: &Document) -> Vec<u8> {
    let mut out = Vec::new();
    push_string(&mut out, &doc.save_name);
    push_u32(&mut out, doc.members.len());
    for m in &doc.members {
        push_member(&mut out, m);
    }
    push_u32(&mut out, doc.subs.len());
    for (name, members) in &doc.subs {
        push_string(&mut out, name);
        push_u32(&mut out, members.len());
        for m in members {
            push_member(&mut out, m);
        }
    }
    out.extend_from_slice(&doc.tail);
    out
}

fn push_string(out: &mut Vec<u8>, s: &str) {
    push_u32(out, s.len());
    out.extend_from_slice(s.as_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: usize) {
    out.extend_from_slice(&u32::try_from(v).unwrap_or(u32::MAX).to_le_bytes());
}

fn push_member(out: &mut Vec<u8>, m: &RawMember) {
    for part in &m.parts {
        push_string(out, part);
    }
    out.extend_from_slice(&m.offset.to_le_bytes());
    out.extend_from_slice(&m.stored.to_le_bytes());
    out.extend_from_slice(&m.plain.to_le_bytes());
    out.extend_from_slice(&m.stamps);
}

fn read_raw_members(frame: &[u8], mut pos: usize, count: usize) -> Option<(Vec<RawMember>, usize)> {
    let mut out = Vec::new();
    for _ in 0..count {
        let mut parts = Vec::new();
        loop {
            let (part, next) = read_string(frame, pos)?;
            pos = next;
            let done = part.starts_with('.');
            parts.push(part);
            if done {
                break;
            }
            if parts.len() >= MAX_NAME_PARTS {
                return None;
            }
        }
        let offset = read_u64(frame, pos)?;
        let stored = read_u64(frame, pos + 8)?;
        let plain = read_u64(frame, pos + 16)?;
        let stamps = frame.get(pos + 24..pos.checked_add(40)?)?;
        pos += 40;
        out.push(RawMember {
            parts,
            offset,
            stored,
            plain,
            stamps: <[u8; 16]>::try_from(stamps).ok()?,
        });
    }
    Some((out, pos))
}

fn read_string(frame: &[u8], pos: usize) -> Option<(String, usize)> {
    let len = read_u32(frame, pos)? as usize;
    if len > MAX_NAME {
        return None;
    }
    let start = pos.checked_add(4)?;
    let bytes = frame.get(start..start.checked_add(len)?)?;
    let s = std::str::from_utf8(bytes).ok()?;
    Some((s.to_owned(), start + len))
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn read_u64(b: &[u8], at: usize) -> Option<u64> {
    let s = b.get(at..at.checked_add(8)?)?;
    Some(u64::from_le_bytes(<[u8; 8]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn push_string(v: &mut Vec<u8>, s: &str) {
        v.extend_from_slice(&(s.len() as u32).to_le_bytes());
        v.extend_from_slice(s.as_bytes());
    }

    fn push_member(v: &mut Vec<u8>, stem: &str, ext: &str, offset: u64, stored: u64, plain: u64) {
        push_string(v, stem);
        push_string(v, ext);
        v.extend_from_slice(&offset.to_le_bytes());
        v.extend_from_slice(&stored.to_le_bytes());
        v.extend_from_slice(&plain.to_le_bytes());
        v.extend_from_slice(&[0u8; 16]);
    }

    fn manifest() -> Vec<u8> {
        // Shaped like the reference save: name, members, one sub-archive,
        // trailing zero u32 — with the sub-archive's members interleaving the
        // top-level ones by offset, as rgman's really do.
        let mut v = Vec::new();
        push_string(&mut v, "Career");
        v.extend_from_slice(&2u32.to_le_bytes());
        push_member(&mut v, "game_info", ".dat", 0, 100, 700);
        push_member(&mut v, "scout_man", ".dat", 300, 50, 400);
        v.extend_from_slice(&1u32.to_le_bytes());
        push_string(&mut v, "rgman");
        v.extend_from_slice(&1u32.to_le_bytes());
        push_member(&mut v, "rule_group", ".dat", 100, 200, 900);
        v.extend_from_slice(&0u32.to_le_bytes());
        v
    }

    #[test]
    fn reads_members_sorted_by_offset() {
        let members = read_manifest(&manifest()).unwrap();
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["game_info.dat", "rgman/rule_group.dat", "scout_man.dat"]);
        assert_eq!(frame_index_of(&members, "scout_man.dat"), Some(2));
        let scout = members.get(2).unwrap();
        assert_eq!((scout.offset, scout.stored, scout.plain), (300, 50, 400));
    }

    #[test]
    fn a_frame_that_is_not_a_manifest_reads_as_none() {
        // A data frame starts `03 01 "tad."` — the 0x74616401 read as a name
        // length blows past MAX_NAME immediately.
        let frame = [0x03, 0x01, b't', b'a', b'd', b'.', 0x32, 0x00, 0xb2, 0x09];
        assert_eq!(read_manifest(&frame), None);
        assert_eq!(read_manifest(&[]), None);
    }

    #[test]
    fn tolerates_truncation_anywhere() {
        let full = manifest();
        for cut in 0..full.len() {
            let _ = read_manifest(full.get(..cut).unwrap());
        }
    }

    #[test]
    fn parse_then_serialize_reproduces_the_bytes() {
        let original = manifest();
        let doc = parse_document(&original).unwrap();
        assert_eq!(doc.save_name, "Career");
        assert_eq!(doc.members.len(), 2);
        assert_eq!(doc.subs.len(), 1);
        assert_eq!(doc.tail, vec![0u8; 4]);
        assert_eq!(serialize(&doc), original);
    }

    #[test]
    fn members_mut_reaches_every_section() {
        let mut doc = parse_document(&manifest()).unwrap();
        for m in doc.members_mut() {
            m.offset += 1000;
        }
        let offsets: Vec<u64> = doc.members_all().map(|m| m.offset).collect();
        assert_eq!(offsets, vec![1000, 1300, 1100]);
    }
}
