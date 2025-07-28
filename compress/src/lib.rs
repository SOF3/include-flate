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
    io::{self, BufRead, BufReader, Read, Seek, Write},
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
pub enum FlateCompressionError {
    #[cfg(feature = "deflate")]
    DeflateError(io::Error),
    #[cfg(feature = "zstd")]
    ZstdError(io::Error),
    IoError(io::Error),
}

impl From<io::Error> for FlateCompressionError {
    fn from(err: io::Error) -> Self {
        FlateCompressionError::IoError(err)
    }
}

impl fmt::Display for FlateCompressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "deflate")]
            FlateCompressionError::DeflateError(err) => write!(f, "Deflate error: {}", err),
            #[cfg(feature = "zstd")]
            FlateCompressionError::ZstdError(err) => write!(f, "Zstd error: {}", err),
            FlateCompressionError::IoError(err) => write!(f, "I/O error: {}", err),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CompressionMethod {
    #[cfg(feature = "deflate")]
    Deflate,
    #[cfg(feature = "zstd")]
    Zstd,
}

impl CompressionMethod {
    pub fn encoder<'a, W: BufRead + Write + Seek + 'a>(
        &'a self,
        write: W,
    ) -> Result<FlateEncoder<W>, FlateCompressionError> {
        FlateEncoder::new(*self, write)
    }

    pub fn decoder<'a, R: ReadSeek + 'a>(
        &'a self,
        read: R,
    ) -> Result<FlateDecoder<'a>, FlateCompressionError> {
        FlateDecoder::new(*self, Box::new(read))
    }
}

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

pub enum FlateEncoder<W: Write> {
    #[cfg(feature = "deflate")]
    Deflate(DeflateEncoder<W>),
    #[cfg(feature = "zstd")]
    Zstd(ZstdEncoder<'static, W>),
}

impl<'a, W: BufRead + Write + Seek + 'a> FlateEncoder<W> {
    pub fn new(
        method: CompressionMethod,
        write: W,
    ) -> Result<FlateEncoder<W>, FlateCompressionError> {
        match method {
            #[cfg(feature = "deflate")]
            CompressionMethod::Deflate => Ok(FlateEncoder::Deflate(DeflateEncoder::new(write))),
            #[cfg(feature = "zstd")]
            CompressionMethod::Zstd => ZstdEncoder::new(write, 0)
                .map(FlateEncoder::Zstd)
                .map_err(FlateCompressionError::ZstdError),
        }
    }
}

impl<'a, W: Write + 'a> Write for FlateEncoder<W> {
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

impl<'a, W: Write + 'a> FlateEncoder<W> {
    fn finish_encode(self) -> Result<W, FlateCompressionError> {
        match self {
            #[cfg(feature = "deflate")]
            FlateEncoder::Deflate(encoder) => encoder
                .finish()
                .into_result()
                .map_err(FlateCompressionError::DeflateError),
            #[cfg(feature = "zstd")]
            FlateEncoder::Zstd(encoder) => {
                encoder.finish().map_err(FlateCompressionError::ZstdError)
            }
        }
    }
}

pub trait ReadSeek: BufRead + Seek {}

impl<T: BufRead + Seek> ReadSeek for T {}

pub enum FlateDecoder<'a> {
    #[cfg(feature = "deflate")]
    Deflate(DeflateDecoder<Box<dyn BufRead + 'a>>),
    #[cfg(feature = "zstd")]
    Zstd(ZstdDecoder<'a, BufReader<Box<dyn BufRead + 'a>>>),
}

impl<'a> FlateDecoder<'a> {
    pub fn new(
        method: CompressionMethod,
        read: Box<dyn BufRead + 'a>,
    ) -> Result<FlateDecoder<'a>, FlateCompressionError> {
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

impl<'a> Read for FlateDecoder<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "deflate")]
            FlateDecoder::Deflate(decoder) => decoder.read(buf),
            #[cfg(feature = "zstd")]
            FlateDecoder::Zstd(decoder) => decoder.read(buf),
        }
    }
}

pub fn apply_compression<R: Sized, W: Sized + BufRead + Seek>(
    reader: &mut R,
    writer: &mut W,
    method: CompressionMethod,
) -> Result<(), FlateCompressionError>
where
    R: Read,
    W: Write,
{
    let mut encoder = method.encoder(writer)?;
    io::copy(reader, &mut encoder)?;
    encoder.finish_encode().map(|_| ())
}

pub fn apply_decompression<R: Sized + BufRead + Seek, W: Sized>(
    reader: &mut R,
    writer: &mut W,
    method: CompressionMethod,
) -> Result<(), FlateCompressionError>
where
    R: Read,
    W: Write,
{
    let mut decoder = method.decoder(reader)?;
    io::copy(&mut decoder, writer)?;
    Ok(())
}
