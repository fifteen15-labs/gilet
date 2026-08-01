//! The interned string table.
//!
//! Names are not stored (only) inline in person records — they reference a
//! table of length-prefixed UTF-8 entries: `u32 id, u32 byte_length, bytes`.
//! The table is made of *sections* whose id spaces overlap: forenames first,
//! then surnames, then common names ("Juanito"), each restarting its ids. A
//! flat id→string map therefore silently resolves forename ids against
//! surname strings; the section must be known.
//!
//! Section identity is positional: of the sections large enough to be name
//! pools, the first is forenames, the second surnames, the third common names.
//! Verified against the reference save, where Haaland's forename id resolves to
//! "Erling" only in the first large section and his surname id to "Haaland"
//! only in the second.
//!
//! The table also fixes where person records begin: they start immediately
//! after it, which is why [`StringTable::end_offset`] is exposed.

use std::collections::HashMap;

/// Upper bound for a plausible string id. The reference save tops out around
/// 600k; anything bigger is noise, not an entry.
const MAX_ID: u32 = 5_000_000;

/// Longest entry accepted. Real names never approach this.
const MAX_LEN: usize = 250;

/// A run of consecutive entries must be at least this long to count as part of
/// the table. Stray byte patterns produce short false chains all over a 105 MB
/// frame; no real section is anywhere near this small.
const MIN_RUN: usize = 1024;

/// A section must hold at least this many entries to be one of the three name
/// pools. The reference save's pools hold 92k-595k entries.
const MIN_SECTION: usize = 10_000;

/// The name pools of a parsed string table.
#[derive(Debug, Clone)]
pub struct StringTable {
    pub forenames: HashMap<u32, String>,
    pub surnames: HashMap<u32, String>,
    pub common_names: HashMap<u32, String>,
    /// Offset just past the table. Person records start here.
    pub end_offset: usize,
}

/// Scans a frame for the string table.
///
/// Returns `None` when no region of the frame contains the three name pools —
/// which is what happens on a frame that is not the main database.
#[must_use]
pub fn scan_strings(frame: &[u8]) -> Option<StringTable> {
    let mut sections: Vec<HashMap<u32, String>> = Vec::new();
    let mut end_offset = 0usize;

    let mut at = 0usize;
    while at + 8 < frame.len() {
        let Some((entries, run_end)) = walk_run(frame, at) else {
            at += 1;
            continue;
        };
        // split the run into sections wherever the id sequence restarts
        let mut current: HashMap<u32, String> = HashMap::new();
        let mut last_id: Option<u32> = None;
        for (id, s) in entries {
            if last_id.is_some_and(|prev| id < prev) && !current.is_empty() {
                sections.push(std::mem::take(&mut current));
            }
            current.insert(id, s);
            last_id = Some(id);
        }
        if !current.is_empty() {
            sections.push(current);
        }
        end_offset = run_end;
        at = run_end;
    }

    let mut pools: Vec<HashMap<u32, String>> = sections
        .into_iter()
        .filter(|s| s.len() >= MIN_SECTION)
        .collect();
    if pools.len() < 3 {
        return None;
    }
    let common_names = pools.remove(2);
    let surnames = pools.remove(1);
    let forenames = pools.remove(0);
    Some(StringTable {
        forenames,
        surnames,
        common_names,
        end_offset,
    })
}

/// Walks consecutive entries starting at `at`. Returns the entries and the
/// offset just past the run, or `None` when the run is too short to be table.
fn walk_run(frame: &[u8], at: usize) -> Option<(Vec<(u32, String)>, usize)> {
    let mut entries = Vec::new();
    let mut p = at;
    while let Some((id, s, next)) = read_entry(frame, p) {
        entries.push((id, s));
        p = next;
    }
    (entries.len() >= MIN_RUN).then_some((entries, p))
}

fn read_entry(frame: &[u8], at: usize) -> Option<(u32, String, usize)> {
    let id = read_u32(frame, at)?;
    if id > MAX_ID {
        return None;
    }
    let len = read_u32(frame, at + 4)? as usize;
    if len > MAX_LEN {
        return None;
    }
    let start = at + 8;
    let raw = frame.get(start..start.checked_add(len)?)?;
    let s = std::str::from_utf8(raw).ok()?;
    Some((id, s.to_owned(), start + len))
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn entry(id: u32, s: &str) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend_from_slice(&(s.len() as u32).to_le_bytes());
        v.extend_from_slice(s.as_bytes());
        v
    }

    /// Three sections big enough to be pools, ids restarting between them.
    fn table() -> Vec<u8> {
        let mut v = Vec::new();
        for i in 0..MIN_SECTION as u32 {
            v.extend(entry(i + 1, &format!("Fore{i}")));
        }
        for i in 0..MIN_SECTION as u32 {
            v.extend(entry(i + 1, &format!("Sur{i}")));
        }
        for i in 0..MIN_SECTION as u32 {
            v.extend(entry(i + 1, &format!("Nick{i}")));
        }
        v
    }

    #[test]
    fn splits_sections_on_id_restart_and_orders_pools() {
        let buf = table();
        let t = scan_strings(&buf).unwrap();
        assert_eq!(t.forenames.get(&1).unwrap(), "Fore0");
        assert_eq!(t.surnames.get(&1).unwrap(), "Sur0");
        assert_eq!(t.common_names.get(&1).unwrap(), "Nick0");
        assert_eq!(t.end_offset, buf.len());
    }

    #[test]
    fn ignores_a_frame_without_a_table() {
        assert!(scan_strings(&[0u8; 4096]).is_none());
    }

    #[test]
    fn a_short_chain_is_not_a_table() {
        let mut v = Vec::new();
        for i in 0..100u32 {
            v.extend(entry(i, "Short"));
        }
        assert!(scan_strings(&v).is_none());
    }

    #[test]
    fn survives_truncation_anywhere() {
        let buf = table();
        for cut in (0..buf.len()).step_by(997) {
            let _ = scan_strings(buf.get(..cut).unwrap());
        }
    }
}
