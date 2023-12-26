// include-flate
// Copyright (C) SOFe, kkent030315
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

include!("../test_util.rs");

use include_flate::flate;
use include_flate_codegen::deflate_file;

// Compression method is defaulted to deflate.
flate!(pub static DATA_DEFLATE1: [u8] from "assets/random.dat");

flate!(pub static DATA_DEFLATE2: [u8] from "assets/random.dat" with deflate);
flate!(pub static DATA_ZSTD: [u8] from "assets/random.dat" with zstd);

#[test]
fn test() {
    verify_with(
        "random.dat",
        deflate_file!("assets/random.dat"),
        CompressionMethod::Deflate,
    );
    verify_with(
        "random.dat",
        deflate_file!("assets/random.dat" deflate),
        CompressionMethod::Deflate,
    );
    verify_with(
        "random.dat",
        deflate_file!("assets/random.dat" zstd),
        CompressionMethod::Zstd,
    );
}
