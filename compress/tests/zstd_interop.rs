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

//! The two Zstandard backends read each other's output: a crate built with
//! `zstd-rust` decodes what a `zstd` build embedded, and the reverse.

#![cfg(all(feature = "zstd", feature = "zstd-rust"))]

use std::io::Read;

use include_flate_compress::{CompressionMethod, apply_compression, apply_decompression};

fn sample() -> Vec<u8> {
    (0..200_000u32)
        .flat_map(|i| format!("{} {} ", i % 977, i % 13).into_bytes())
        .collect()
}

#[test]
fn a_c_zstd_frame_decodes_with_the_rust_backend() {
    let data = sample();
    let frame = zstd::encode_all(&data[..], 0).unwrap();

    let mut decoded = Vec::new();
    apply_decompression(&frame[..], &mut decoded, CompressionMethod::Zstd).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn a_rust_backend_frame_decodes_with_c_zstd() {
    let data = sample();
    let mut frame = Vec::new();
    apply_compression(&mut &data[..], &mut frame, CompressionMethod::Zstd).unwrap();

    let mut decoded = Vec::new();
    zstd::Decoder::new(&frame[..])
        .unwrap()
        .read_to_end(&mut decoded)
        .unwrap();
    assert_eq!(decoded, data);
}
