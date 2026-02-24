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

flate!(pub static DATA1: [u8] from "assets/009f.dat");
#[cfg(feature = "deflate")]
flate!(pub static DATA2: [u8] from "assets/009f.dat" with deflate);
#[cfg(feature = "zstd")]
flate!(pub static DATA3: [u8] from "assets/009f.dat" with zstd);
#[cfg(feature = "deflate")]
flate!(pub static DATA4: IFlate from "assets/009f.dat" with deflate);
#[cfg(feature = "zstd")]
flate!(pub static DATA5: IFlate from "assets/009f.dat" with zstd);

#[test]
fn test() {
    verify("009f.dat", &DATA1);
    #[cfg(feature = "deflate")]
    verify("009f.dat", &DATA2);
    #[cfg(feature = "zstd")]
    verify("009f.dat", &DATA3);
    #[cfg(feature = "deflate")]
    verify_iflate("009f.dat", CompressionMethod::Deflate, &DATA4);
    #[cfg(feature = "zstd")]
    verify_iflate("009f.dat", CompressionMethod::Zstd, &DATA5);
}
