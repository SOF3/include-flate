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

flate!(pub static DATA1: [u8] from "assets/random.dat");
flate!(pub static DATA2: [u8] from "assets/random.dat" with deflate);
flate!(pub static DATA3: [u8] from "assets/random.dat" with zstd);

#[test]
fn test() {
    verify("random.dat", &DATA1);
    verify("random.dat", &DATA2);
    verify("random.dat", &DATA3);
}
