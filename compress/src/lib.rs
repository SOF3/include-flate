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

use std::{
    fmt,
    io::{self, BufRead, BufReader, Read, Seek, Write},
    str::FromStr,
};

use libflate::deflate::Decoder as DeflateDecoder;
use libflate::deflate::Encoder as DeflateEncoder;
use zstd::Decoder as ZstdDecoder;
use zstd::Encoder as ZstdEncoder;

#[derive(Debug)]
pub enum FlateCompressionError {
    DeflateError(io::Error),
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
            FlateCompressionError::DeflateError(err) => write!(f, "Deflate error: {}", err),
            FlateCompressionError::ZstdError(err) => write!(f, "Zstd error: {}", err),
            FlateCompressionError::IoError(err) => write!(f, "I/O error: {}", err),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CompressionMethod {
    Deflate,
    Zstd,
}

impl CompressionMethod {
    pub fn encoder<'a, W: BufRead + Write + Seek + 'a>(
        &'a self,
        write: W,
    ) -> Result<FlateEncoder<'a, W>, FlateCompressionError> {
        FlateEncoder::new(*self, write)
    }

    pub fn decoder<'a, R: ReadSeek + 'a>(
        &'a self,
        read: R,
    ) -> Result<FlateDecoder<'a>, FlateCompressionError> {
        FlateDecoder::new(*self, Box::new(read))
    }
}

impl Default for CompressionMethod {
    fn default() -> Self {
        Self::Deflate
    }
}

impl FromStr for CompressionMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_lowercase() {
            "deflate" => Ok(CompressionMethod::Deflate),
            "zstd" => Ok(CompressionMethod::Zstd),
            _ => Err(format!("Unknown compression method: {}", s).into()),
        }
    }
}

pub enum FlateEncoder<'a, W: Write + 'a> {
    Deflate(DeflateEncoder<W>),
    Zstd(ZstdEncoder<'a, W>),
}

impl<'a, W: BufRead + Write + Seek + 'a> FlateEncoder<'a, W> {
    pub fn new(
        method: CompressionMethod,
        write: W,
    ) -> Result<FlateEncoder<'a, W>, FlateCompressionError> {
        match method {
            CompressionMethod::Deflate => Ok(FlateEncoder::Deflate(DeflateEncoder::new(write))),
            CompressionMethod::Zstd => ZstdEncoder::new(write, 0)
                .map(FlateEncoder::Zstd)
                .map_err(|e| FlateCompressionError::ZstdError(e)),
        }
    }
}

impl<'a, W: Write + 'a> Write for FlateEncoder<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            FlateEncoder::Deflate(encoder) => encoder.write(buf),
            FlateEncoder::Zstd(encoder) => encoder.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            FlateEncoder::Deflate(encoder) => encoder.flush(),
            FlateEncoder::Zstd(encoder) => encoder.flush(),
        }
    }
}

impl<'a, W: Write + 'a> FlateEncoder<'a, W> {
    fn finish_encode(self) -> Result<W, FlateCompressionError> {
        match self {
            FlateEncoder::Deflate(encoder) => encoder
                .finish()
                .into_result()
                .map_err(|e| FlateCompressionError::DeflateError(e)),
            FlateEncoder::Zstd(encoder) => encoder
                .finish()
                .map_err(|e| FlateCompressionError::ZstdError(e)),
        }
    }
}

pub trait ReadSeek: BufRead + Seek {}

impl<T: BufRead + Seek> ReadSeek for T {}

pub enum FlateDecoder<'a> {
    Deflate(DeflateDecoder<Box<dyn BufRead + 'a>>),
    Zstd(ZstdDecoder<'a, BufReader<Box<dyn BufRead + 'a>>>),
}

impl<'a> FlateDecoder<'a> {
    pub fn new(
        method: CompressionMethod,
        read: Box<dyn BufRead + 'a>,
    ) -> Result<FlateDecoder<'a>, FlateCompressionError> {
        match method {
            CompressionMethod::Deflate => Ok(FlateDecoder::Deflate(DeflateDecoder::new(read))),
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
            FlateDecoder::Deflate(decoder) => decoder.read(buf),
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
