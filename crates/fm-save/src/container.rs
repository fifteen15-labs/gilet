use crate::error::{Error, Result, ZstdError};
use zstd_safe::{DCtx, InBuffer, OutBuffer};

/// `"fmf."`, at offset 2. Bytes 0..2 are a version pair (`02 01` in every save
/// seen so far) and are deliberately not validated.
pub const MAGIC: [u8; 4] = *b"fmf.";

/// Where the first zstd frame begins. The 26-byte header carries no frame
/// index, so this is a constant rather than something we read.
pub const HEADER_LEN: usize = 26;

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// One decompressed block. `offset` is into the original file, which makes a
/// frame traceable back to the bytes on disk when reverse-engineering.
#[derive(Debug, Clone)]
pub struct Frame {
    pub index: usize,
    pub offset: usize,
    pub compressed_len: usize,
    pub data: Vec<u8>,
}

/// Splits a save into its decompressed frames.
///
/// The frames sit back to back with no index, so each one is decoded to learn
/// how many input bytes it consumed, and that lands us on the next. Scanning
/// for the zstd magic instead does not work: the byte sequence also occurs
/// inside compressed payloads (1,760 occurrences for 1,215 real frames in the
/// reference save), so it is used only to resynchronise after a bad frame.
///
/// # Errors
/// Returns [`Error::BadMagic`] if this is not an `fmf.` file, [`Error::TooShort`]
/// if it cannot hold a header, and [`Error::Decompress`] if a frame is corrupt.
pub fn read_frames(bytes: &[u8]) -> Result<Vec<Frame>> {
    let header = bytes.get(..HEADER_LEN).ok_or(Error::TooShort { len: bytes.len() })?;
    let found = header.get(2..6).and_then(|s| <[u8; 4]>::try_from(s).ok());
    match found {
        Some(m) if m == MAGIC => {}
        Some(m) => return Err(Error::BadMagic { expected: MAGIC, found: m }),
        None => return Err(Error::TooShort { len: bytes.len() }),
    }

    let mut frames = Vec::new();
    let mut offset = HEADER_LEN;

    while let Some(rest) = bytes.get(offset..) {
        if rest.len() < ZSTD_MAGIC.len() {
            break;
        }
        let (data, consumed) = match decode_one(rest) {
            Ok(pair) => pair,
            Err(source) => {
                // Resynchronise on the next plausible frame start. A trailing
                // non-frame tail is normal, so running out is not an error.
                match find_next_frame(bytes, offset + 1) {
                    Some(next) => {
                        offset = next;
                        continue;
                    }
                    None => {
                        return Err(Error::Decompress {
                            frame: frames.len(),
                            offset,
                            source,
                        })
                    }
                }
            }
        };
        if consumed == 0 {
            return Err(Error::StalledFrame { frame: frames.len(), offset });
        }
        frames.push(Frame {
            index: frames.len(),
            offset,
            compressed_len: consumed,
            data,
        });
        offset += consumed;
    }

    Ok(frames)
}

fn find_next_frame(bytes: &[u8], from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(ZSTD_MAGIC.len())
        .position(|w| w == ZSTD_MAGIC)
        .map(|p| from + p)
}

/// Decodes exactly one frame, returning the payload and the number of input
/// bytes it used. Stopping at the frame boundary is the whole point: the `zstd`
/// CLI and stream readers either read across frames or reject the tail
/// outright.
fn decode_one(src: &[u8]) -> std::result::Result<(Vec<u8>, usize), ZstdError> {
    let mut ctx = DCtx::create();
    let mut input = InBuffer::around(src);
    // zstd-safe implements its output trait for slices, not `Vec`, so decode
    // through a reusable chunk and append. Frames range from 26 bytes to 105 MB
    // in the reference save, so the payload itself grows on demand.
    let mut chunk = vec![0u8; 256 * 1024];
    let mut out: Vec<u8> = Vec::new();

    loop {
        let (hint, produced) = {
            let mut output = OutBuffer::around(chunk.as_mut_slice());
            let hint = ctx
                .decompress_stream(&mut output, &mut input)
                .map_err(|code| ZstdError(zstd_safe::get_error_name(code).to_owned()))?;
            (hint, output.pos())
        };
        let written = chunk
            .get(..produced)
            .ok_or_else(|| ZstdError("decoder reported more output than the buffer holds".to_owned()))?;
        out.extend_from_slice(written);

        // Zero means the frame is complete; anything else is a size hint for
        // the next call.
        if hint == 0 {
            break;
        }
        if produced == 0 && input.pos() == src.len() {
            return Err(ZstdError("truncated frame: input exhausted".to_owned()));
        }
    }

    let consumed = input.pos();
    Ok((out, consumed))
}
