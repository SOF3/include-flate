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

include!("../test_util.rs");

use include_flate::flate;

pub static DATA_RAW: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/base64.txt"));

flate!(pub static DATA1: str from "assets/base64.txt");
flate!(pub static DATA2: str from "assets/base64.txt" with deflate);
flate!(pub static DATA3: str from "assets/base64.txt" with zstd);
flate!(pub static DATA4: IFlate from "assets/base64.txt" with deflate);
flate!(pub static DATA5: IFlate from "assets/base64.txt" with zstd);

#[test]
fn test() {
    verify_str("base64.txt", &DATA1);
    verify_str("base64.txt", &DATA2);
    verify_str("base64.txt", &DATA3);
    // verify_iflate("base64.txt", "deflate", &DATA4); // FAIL
    verify_iflate("base64.txt", "zstd", &DATA5);
}
