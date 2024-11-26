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
use include_flate_codegen::deflate_if;

// `assets/one.txt` is a file containing the string `1`.
// By the nature of compression, small datas will likely be larger after compression.

flate!(pub static ONE_ALWAYS: [u8] from "assets/one.txt" if always);
flate!(pub static ONE_DEFLATE_ALWAYS: [u8] from "assets/one.txt" with deflate if always);
flate!(pub static ONE_ZSTD_ALWAYS: [u8] from "assets/one.txt" with zstd if always);

flate!(pub static ONE_LESS_THAN_ORIGINAL: [u8] from "assets/one.txt" if less_than_original);
flate!(pub static ONE_DEFLATE_LESS_THAN_ORIGINAL: [u8] from "assets/one.txt" with deflate if less_than_original);
flate!(pub static ONE_ZSTD_LESS_THAN_ORIGINAL: [u8] from "assets/one.txt" with zstd if less_than_original);

#[test]
fn test() {
    verify("one.txt", &ONE_ALWAYS);
    verify("one.txt", &ONE_DEFLATE_ALWAYS);
    verify("one.txt", &ONE_ZSTD_ALWAYS);

    verify("one.txt", &ONE_LESS_THAN_ORIGINAL);
    verify("one.txt", &ONE_DEFLATE_LESS_THAN_ORIGINAL);
    verify("one.txt", &ONE_ZSTD_LESS_THAN_ORIGINAL);

    assert_eq!(deflate_if!("assets/one.txt" zstd always), true);
    assert_eq!(deflate_if!("assets/one.txt" deflate always), true);

    // The compressed data is larger than the original data (as expected),
    // so it should not be deflated.
    assert_eq!(deflate_if!("assets/one.txt" zstd less_than_original), false);
    assert_eq!(
        deflate_if!("assets/one.txt" deflate less_than_original),
        false
    );

    // The compressed data is bigger than the original data (as expected),
    assert_eq!(
        deflate_if!("assets/one.txt" zstd compression_ratio_more_than 10%),
        false
    );
    assert_eq!(
        deflate_if!("assets/one.txt" deflate compression_ratio_more_than 10%),
        false
    );
}
