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

//! `Zstd` and `ZstdRust` share the Zstandard frame format, so each one
//! decodes what the other encoded.

#![cfg(all(feature = "zstd", feature = "zstd-rust"))]

use super::{CompressionMethod, apply_compression, apply_decompression};

fn sample() -> Vec<u8> {
    (0..200_000u32)
        .flat_map(|i| format!("{} {} ", i % 977, i % 13).into_bytes())
        .collect()
}

fn round_trip(encode: CompressionMethod, decode: CompressionMethod) {
    let data = sample();
    let mut frame = Vec::new();
    apply_compression(&mut &data[..], &mut frame, encode).unwrap();

    let mut decoded = Vec::new();
    apply_decompression(&frame[..], &mut decoded, decode).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn a_zstd_frame_decodes_with_zstd_rust() {
    round_trip(CompressionMethod::Zstd, CompressionMethod::ZstdRust);
}

#[test]
fn a_zstd_rust_frame_decodes_with_zstd() {
    round_trip(CompressionMethod::ZstdRust, CompressionMethod::Zstd);
}
