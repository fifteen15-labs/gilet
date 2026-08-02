use std::fmt;

/// Everything that can go wrong reading a save. A malformed file is an error,
/// never a panic — see the `unwrap_used` ban in the workspace lints.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a Football Manager save: expected magic {expected:?} at offset 2, found {found:?}")]
    BadMagic { expected: [u8; 4], found: [u8; 4] },

    #[error("file is {len} bytes, too short to contain a save header")]
    TooShort { len: usize },

    #[error("zstd error decoding frame {frame} at offset {offset}: {source}")]
    Decompress {
        frame: usize,
        offset: usize,
        source: ZstdError,
    },

    #[error("frame {frame} at offset {offset} consumed no input; refusing to loop")]
    StalledFrame { frame: usize, offset: usize },

    #[error("save archive cannot be rewritten: {reason}")]
    Archive { reason: String },
}

/// zstd-safe reports failures as a bare code plus a static description, which
/// does not implement `std::error::Error`. Wrap it so it composes.
#[derive(Debug)]
pub struct ZstdError(pub String);

impl fmt::Display for ZstdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ZstdError {}

pub type Result<T> = std::result::Result<T, Error>;
