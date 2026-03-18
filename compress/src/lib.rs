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

#[cfg(not(any(feature = "zstd", feature = "deflate")))]
compile_error!("You must enable either the `deflate` or `zstd` feature.");

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

#[derive(Debug)]
pub enum CompressionError {
    #[cfg(feature = "deflate")]
    DeflateError(io::Error),
    #[cfg(feature = "zstd")]
    ZstdError(io::Error),
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
            CompressionError::IoError(err) => write!(f, "I/O error: {}", err),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompressionMethod {
    #[cfg(feature = "deflate")]
    Deflate,
    #[cfg(feature = "zstd")]
    Zstd,
}

impl CompressionMethod {
    pub fn encoder<W: Write>(
        &self,
        write: W,
    ) -> Result<FlateEncoder<W>, CompressionError> {
        FlateEncoder::new(*self, write)
    }

    pub fn decoder<R: Read>(
        &self,
        read: R,
    ) -> Result<FlateDecoder<R>, CompressionError> {
        FlateDecoder::new(*self, read)
    }
}

#[expect(clippy::derivable_impls, reason = "cfg_attr on defaults could be confusing")]
#[cfg(any(feature = "deflate", feature = "zstd"))]
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
    }
}

impl fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            #[cfg(feature = "deflate")]
            Self::Deflate => "deflate",
            #[cfg(feature = "zstd")]
            Self::Zstd => "zstd",
        })
    }
}

pub enum FlateEncoder<W: Write> {
    #[cfg(feature = "deflate")]
    Deflate(DeflateEncoder<W>),
    #[cfg(feature = "zstd")]
    Zstd(ZstdEncoder<'static, W>),
}

impl<W: Write> FlateEncoder<W> {
    pub fn new(
        method: CompressionMethod,
        write: W,
    ) -> Result<FlateEncoder<W>, CompressionError> {
        match method {
            #[cfg(feature = "deflate")]
            CompressionMethod::Deflate => Ok(FlateEncoder::Deflate(DeflateEncoder::new(write))),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => ZstdEncoder::new(write, 0)
                .map(FlateEncoder::Zstd)
                .map_err(CompressionError::ZstdError),
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
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(feature = "deflate")]
            FlateEncoder::Deflate(encoder) => encoder.flush(),
            #[cfg(feature = "zstd")]
            FlateEncoder::Zstd(encoder) => encoder.flush(),
        }
    }
}

impl<W: Write> FlateEncoder<W> {
    fn finish_encode(self) -> Result<W, CompressionError> {
        match self {
            #[cfg(feature = "deflate")]
            FlateEncoder::Deflate(encoder) => encoder
                .finish()
                .into_result()
                .map_err(CompressionError::DeflateError),
            #[cfg(feature = "zstd")]
            FlateEncoder::Zstd(encoder) => {
                encoder.finish().map_err(CompressionError::ZstdError)
            }
        }
    }
}

pub enum FlateDecoder<R> {
    #[cfg(feature = "deflate")]
    Deflate(DeflateDecoder<R>),
    #[cfg(feature = "zstd")]
    Zstd(ZstdDecoder<'static, std::io::BufReader<R>>),
}

impl<R: Read> FlateDecoder<R> {
    pub fn new(
        method: CompressionMethod,
        read: R,
    ) -> Result<FlateDecoder<R>, CompressionError> {
        match method {
            #[cfg(feature = "deflate")]
            CompressionMethod::Deflate => Ok(FlateDecoder::Deflate(DeflateDecoder::new(read))),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => {
                let decoder = ZstdDecoder::new(read)?;
                Ok(FlateDecoder::Zstd(decoder))
            }
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
    encoder.finish_encode().map(|_| ())
}

pub fn apply_decompression(
    reader: impl Read,
    mut writer: impl Write,
    method: CompressionMethod,
) -> Result<(), CompressionError>
{
    let mut decoder = method.decoder(reader)?;
    io::copy(&mut decoder, &mut writer)?;
    Ok(())
}
