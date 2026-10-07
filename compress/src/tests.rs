// include-flate
// Copyright (C) SOFe
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

#![cfg(feature = "zstd-rust")]

use super::{CompressionMethod, apply_compression, decompress_slice};

fn sample() -> Vec<u8> {
    (0..200_000u32)
        .flat_map(|i| format!("{} {} ", i % 977, i % 13).into_bytes())
        .collect()
}

fn encode(data: &[u8], method: CompressionMethod) -> Vec<u8> {
    let mut frame = Vec::new();
    apply_compression(&mut &data[..], &mut frame, method).unwrap();
    frame
}

/// `Zstd` and `ZstdRust` share the Zstandard frame format, so each one
/// decodes what the other encoded, through both decoding entry points.
#[cfg(feature = "zstd")]
fn round_trip(encode_with: CompressionMethod, decode_with: CompressionMethod) {
    use super::apply_decompression;

    let data = sample();
    let frame = encode(&data, encode_with);

    let mut decoded = Vec::new();
    apply_decompression(&frame[..], &mut decoded, decode_with).unwrap();
    assert_eq!(decoded, data);

    assert_eq!(decompress_slice(&frame, decode_with).unwrap(), data);
}

#[cfg(feature = "zstd")]
#[test]
fn a_zstd_frame_decodes_with_zstd_rust() {
    round_trip(CompressionMethod::Zstd, CompressionMethod::ZstdRust);
}

#[cfg(feature = "zstd")]
#[test]
fn a_zstd_rust_frame_decodes_with_zstd() {
    round_trip(CompressionMethod::ZstdRust, CompressionMethod::Zstd);
}

/// The one-shot decode sizes its output from `Frame_Content_Size`, so a
/// `ZstdRust` frame must declare it.
#[test]
fn a_zstd_rust_frame_declares_its_content_size() {
    use structured_zstd::decoding::{FrameContentSize, read_frame_content_size};

    let data = sample();
    let frame = encode(&data, CompressionMethod::ZstdRust);
    assert_eq!(
        read_frame_content_size(&frame).unwrap(),
        FrameContentSize::Known(data.len() as u64)
    );
    assert_eq!(
        decompress_slice(&frame, CompressionMethod::ZstdRust).unwrap(),
        data
    );
}

/// Empty input still makes a frame that declares its (zero) size and decodes
/// back to nothing.
#[test]
fn an_empty_zstd_rust_frame_round_trips() {
    let frame = encode(&[], CompressionMethod::ZstdRust);
    assert!(
        decompress_slice(&frame, CompressionMethod::ZstdRust)
            .unwrap()
            .is_empty()
    );
}

/// `Frame_Content_Size` is optional (RFC 8878 3.1.1.1.2); a frame without it
/// still decodes, through the streaming decoder.
#[test]
fn a_zstd_rust_frame_without_content_size_decodes() {
    use std::io::Write;
    use structured_zstd::decoding::{FrameContentSize, read_frame_content_size};
    use structured_zstd::encoding::{CompressionLevel, StreamingEncoder};

    let data = sample();
    let mut encoder = StreamingEncoder::new(Vec::new(), CompressionLevel::Default);
    encoder.write_all(&data).unwrap();
    let frame = encoder.finish().unwrap();
    assert_eq!(
        read_frame_content_size(&frame).unwrap(),
        FrameContentSize::Unknown
    );

    assert_eq!(
        decompress_slice(&frame, CompressionMethod::ZstdRust).unwrap(),
        data
    );
}

/// A frame whose blocks hold less than its header declares is reported as an
/// error, not returned with a zero-filled tail.
#[test]
fn a_zstd_rust_frame_shorter_than_declared_is_an_error() {
    use structured_zstd::decoding::{FrameContentSize, read_frame_content_size};

    let data = sample();
    let mut frame = encode(&data, CompressionMethod::ZstdRust);

    // RFC 8878 3.1.1.1: magic (4), descriptor (1), then the window descriptor
    // unless Single_Segment_flag is set, the dictionary ID, and the
    // little-endian Frame_Content_Size field.
    let descriptor = frame[4];
    let single_segment = descriptor & 0x20 != 0;
    let dict_id_len = [0, 1, 2, 4][usize::from(descriptor & 0x03)];
    let fcs_len = match descriptor >> 6 {
        0 => usize::from(single_segment),
        1 => 2,
        2 => 4,
        _ => 8,
    };
    assert_eq!(fcs_len, 4, "the sample needs a 4-byte FCS field");
    let at = 5 + usize::from(!single_segment) + dict_id_len;
    let declared = u32::from_le_bytes(frame[at..at + 4].try_into().unwrap());
    frame[at..at + 4].copy_from_slice(&(declared + 1).to_le_bytes());
    assert_eq!(
        read_frame_content_size(&frame).unwrap(),
        FrameContentSize::Known(data.len() as u64 + 1)
    );

    assert!(decompress_slice(&frame, CompressionMethod::ZstdRust).is_err());
}
