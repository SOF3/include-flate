// include-flate
// Copyright (C) SOFe, Kento Oki
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(not(any(feature = "zstd", feature = "zstd-rust", feature = "deflate")))]
compile_error!("You must enable either the `deflate`, `zstd` or `zstd-rust` feature.");

use std::{
    fmt,
    io::{self, Read, Write},
};

#[cfg(feature = "deflate")]
use libflate::deflate::Decoder as DeflateDecoder;
#[cfg(feature = "deflate")]
use libflate::deflate::Encoder as DeflateEncoder;
#[cfg(feature = "zstd")]
use zstd::Decoder as ZstdDecoder;
#[cfg(feature = "zstd")]
use zstd::Encoder as ZstdEncoder;

// The pure-Rust decoder keeps its state inline (the C one holds a pointer to
// it), so it is boxed to keep the enum below the size of its other variants.
#[cfg(feature = "zstd-rust")]
type ZstdRustDecoder<R> =
    Box<structured_zstd::decoding::StreamingDecoder<R, structured_zstd::decoding::FrameDecoder>>;

/// Decodes a Zstandard stream held in memory: every frame, skippable frames
/// skipped (RFC 8878 3).
#[cfg(feature = "zstd-rust")]
fn decode_zstd_rust(bytes: &[u8]) -> io::Result<Vec<u8>> {
    use structured_zstd::decoding::errors::ReadFrameHeaderError;
    use structured_zstd::decoding::{
        FrameContentSize, FrameDecoder, StreamingDecoder, find_frame_compressed_size,
        read_frame_content_size,
    };

    fn invalid(err: impl fmt::Debug) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, format!("{err:?}"))
    }

    // RFC 8878 3: compressed data is one or more frames.
    if bytes.is_empty() {
        return Err(invalid("empty input holds no Zstandard frame"));
    }

    // Walk the frames first: when every one declares Frame_Content_Size
    // (optional, RFC 8878 3.1.1.1.2), the stream decodes in one call straight
    // into a buffer of exactly their total size.
    let mut total = 0u64;
    let mut all_declared = true;
    let mut offset = 0;
    while offset < bytes.len() {
        let frame = &bytes[offset..];
        let len = find_frame_compressed_size(frame).map_err(invalid)?;
        match read_frame_content_size(frame) {
            Ok(size) => {
                if let FrameContentSize::Known(size) = size {
                    // RFC 8878 3.1.1.2: every block takes at least its 3-byte
                    // header on disk and regenerates at most 128 KiB, so a
                    // frame declaring more than that cannot be honest.
                    let max = (len as u64 / 3) * (128 * 1024);
                    if size > max {
                        return Err(invalid(format!(
                            "frame declares {size} bytes but its {len} bytes hold at most {max}"
                        )));
                    }
                    // Each term is bounded by its frame's length as above, so
                    // the sum stays far below u64::MAX.
                    total += size;
                } else {
                    all_declared = false;
                }
            }
            Err(ReadFrameHeaderError::SkipFrame { .. }) => {}
            Err(err) => return Err(invalid(err)),
        }
        offset += len;
    }

    if all_declared {
        let size = usize::try_from(total).map_err(invalid)?;
        let mut out = vec![0; size];
        let written = FrameDecoder::new()
            .decode_all(bytes, &mut out)
            .map_err(invalid)?;
        if written != size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("frames declare {size} bytes but hold {written}"),
            ));
        }
        return Ok(out);
    }

    let mut out = Vec::new();
    StreamingDecoder::new(bytes)
        .map_err(invalid)?
        .read_to_end(&mut out)?;
    Ok(out)
}

/// Collects the whole input and encodes it as one frame on finish, so the
/// frame declares `Frame_Content_Size` and `decompress_slice` can decode it
/// into an exactly sized buffer in one call.
#[cfg(feature = "zstd-rust")]
pub struct ZstdRustEncoder<W: Write> {
    input: Vec<u8>,
    sink: W,
}

#[derive(Debug)]
pub enum CompressionError {
    #[cfg(feature = "deflate")]
    DeflateError(io::Error),
    #[cfg(feature = "zstd")]
    ZstdError(io::Error),
    #[cfg(feature = "zstd-rust")]
    ZstdRustError(io::Error),
    IoError(io::Error),
}

impl From<io::Error> for CompressionError {
    fn from(err: io::Error) -> Self {
        CompressionError::IoError(err)
    }
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "deflate")]
            CompressionError::DeflateError(err) => write!(f, "Deflate error: {}", err),
            #[cfg(feature = "zstd")]
            CompressionError::ZstdError(err) => write!(f, "Zstd error: {}", err),
            #[cfg(feature = "zstd-rust")]
            CompressionError::ZstdRustError(err) => write!(f, "Zstd (pure Rust) error: {}", err),
            CompressionError::IoError(err) => write!(f, "I/O error: {}", err),
        }
    }
}

/// `Zstd` and `ZstdRust` produce and read the same Zstandard frame format:
/// `Zstd` through the C library, `ZstdRust` through the pure-Rust
/// `structured-zstd`, which needs no C toolchain.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompressionMethod {
    #[cfg(feature = "deflate")]
    Deflate,
    #[cfg(feature = "zstd")]
    Zstd,
    #[cfg(feature = "zstd-rust")]
    ZstdRust,
}

impl CompressionMethod {
    pub fn encoder<W: Write>(&self, write: W) -> Result<FlateEncoder<W>, CompressionError> {
        FlateEncoder::new(*self, write)
    }

    pub fn decoder<R: Read>(&self, read: R) -> Result<FlateDecoder<R>, CompressionError> {
        FlateDecoder::new(*self, read)
    }
}

#[expect(
    clippy::derivable_impls,
    reason = "cfg_attr on defaults could be confusing"
)]
#[cfg(any(feature = "deflate", feature = "zstd", feature = "zstd-rust"))]
impl Default for CompressionMethod {
    fn default() -> Self {
        #[cfg(feature = "deflate")]
        {
            Self::Deflate
        }
        #[cfg(all(not(feature = "deflate"), feature = "zstd"))]
        {
            Self::Zstd
        }
        #[cfg(all(not(feature = "deflate"), not(feature = "zstd"), feature = "zstd-rust"))]
        {
            Self::ZstdRust
        }
    }
}

impl fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            #[cfg(feature = "deflate")]
            Self::Deflate => "deflate",
            #[cfg(feature = "zstd")]
            Self::Zstd => "zstd",
            #[cfg(feature = "zstd-rust")]
            Self::ZstdRust => "zstd_rust",
        })
    }
}

pub enum FlateEncoder<W: Write> {
    #[cfg(feature = "deflate")]
    Deflate(DeflateEncoder<W>),
    #[cfg(feature = "zstd")]
    Zstd(ZstdEncoder<'static, W>),
    #[cfg(feature = "zstd-rust")]
    ZstdRust(ZstdRustEncoder<W>),
}

impl<W: Write> FlateEncoder<W> {
    pub fn new(method: CompressionMethod, write: W) -> Result<FlateEncoder<W>, CompressionError> {
        match method {
            #[cfg(feature = "deflate")]
            CompressionMethod::Deflate => Ok(FlateEncoder::Deflate(DeflateEncoder::new(write))),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => ZstdEncoder::new(write, 0)
                .map(FlateEncoder::Zstd)
                .map_err(CompressionError::ZstdError),
            #[cfg(feature = "zstd-rust")]
            CompressionMethod::ZstdRust => Ok(FlateEncoder::ZstdRust(ZstdRustEncoder {
                input: Vec::new(),
                sink: write,
            })),
        }
    }
}

impl<W: Write> Write for FlateEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "deflate")]
            FlateEncoder::Deflate(encoder) => encoder.write(buf),
            #[cfg(feature = "zstd")]
            FlateEncoder::Zstd(encoder) => encoder.write(buf),
            #[cfg(feature = "zstd-rust")]
            FlateEncoder::ZstdRust(encoder) => {
                encoder.input.extend_from_slice(buf);
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(feature = "deflate")]
            FlateEncoder::Deflate(encoder) => encoder.flush(),
            #[cfg(feature = "zstd")]
            FlateEncoder::Zstd(encoder) => encoder.flush(),
            // Nothing is encoded before the frame is finished.
            #[cfg(feature = "zstd-rust")]
            FlateEncoder::ZstdRust(_) => Ok(()),
        }
    }
}

impl<W: Write> FlateEncoder<W> {
    /// Ends the stream and returns the writer. Every method needs this to
    /// write its final bytes; `ZstdRust` writes the whole frame here.
    pub fn finish(self) -> Result<W, CompressionError> {
        match self {
            #[cfg(feature = "deflate")]
            FlateEncoder::Deflate(encoder) => encoder
                .finish()
                .into_result()
                .map_err(CompressionError::DeflateError),
            #[cfg(feature = "zstd")]
            FlateEncoder::Zstd(encoder) => encoder.finish().map_err(CompressionError::ZstdError),
            // `Default` is this backend's counterpart of zstd level 3, which
            // the C backend selects through level 0.
            #[cfg(feature = "zstd-rust")]
            FlateEncoder::ZstdRust(ZstdRustEncoder { input, mut sink }) => {
                let frame = structured_zstd::encoding::compress_slice_to_vec(
                    &input,
                    structured_zstd::encoding::CompressionLevel::Default,
                );
                sink.write_all(&frame)
                    .map_err(CompressionError::ZstdRustError)?;
                Ok(sink)
            }
        }
    }
}

pub enum FlateDecoder<R: Read> {
    #[cfg(feature = "deflate")]
    Deflate(DeflateDecoder<R>),
    #[cfg(feature = "zstd")]
    Zstd(ZstdDecoder<'static, std::io::BufReader<R>>),
    #[cfg(feature = "zstd-rust")]
    ZstdRust(ZstdRustDecoder<R>),
}

impl<R: Read> FlateDecoder<R> {
    pub fn new(method: CompressionMethod, read: R) -> Result<FlateDecoder<R>, CompressionError> {
        match method {
            #[cfg(feature = "deflate")]
            CompressionMethod::Deflate => Ok(FlateDecoder::Deflate(DeflateDecoder::new(read))),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => {
                let decoder = ZstdDecoder::new(read)?;
                Ok(FlateDecoder::Zstd(decoder))
            }
            #[cfg(feature = "zstd-rust")]
            CompressionMethod::ZstdRust => structured_zstd::decoding::StreamingDecoder::new(read)
                .map(|decoder| FlateDecoder::ZstdRust(Box::new(decoder)))
                .map_err(|err| CompressionError::ZstdRustError(io::Error::other(err))),
        }
    }
}

impl<R: Read> Read for FlateDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "deflate")]
            FlateDecoder::Deflate(decoder) => decoder.read(buf),
            #[cfg(feature = "zstd")]
            FlateDecoder::Zstd(decoder) => decoder.read(buf),
            #[cfg(feature = "zstd-rust")]
            FlateDecoder::ZstdRust(decoder) => decoder.read(buf),
        }
    }
}

pub fn apply_compression<R, W>(
    reader: &mut R,
    writer: &mut W,
    method: CompressionMethod,
) -> Result<(), CompressionError>
where
    R: Read,
    W: Write,
{
    let mut encoder = method.encoder(writer)?;
    io::copy(reader, &mut encoder)?;
    encoder.finish().map(|_| ())
}

pub fn apply_decompression(
    reader: impl Read,
    mut writer: impl Write,
    method: CompressionMethod,
) -> Result<(), CompressionError> {
    let mut decoder = method.decoder(reader)?;
    io::copy(&mut decoder, &mut writer)?;
    Ok(())
}

/// Decodes a whole compressed buffer held in memory.
///
/// Meant for data this crate encoded, such as what `flate!` embeds. For
/// `ZstdRust` the output is allocated up front at the size the frames declare,
/// bounded only by what their length could hold, so a crafted input can ask
/// for far more memory than it decodes to; decode untrusted input through
/// [`CompressionMethod::decoder`] instead.
pub fn decompress_slice(
    bytes: &[u8],
    method: CompressionMethod,
) -> Result<Vec<u8>, CompressionError> {
    #[cfg(feature = "zstd-rust")]
    if method == CompressionMethod::ZstdRust {
        return decode_zstd_rust(bytes).map_err(CompressionError::ZstdRustError);
    }

    let mut out = Vec::new();
    apply_decompression(bytes, &mut out, method)?;
    Ok(out)
}

#[cfg(test)]
mod tests;
