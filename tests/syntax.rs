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

use include_flate::flate;

flate!(pub static DATA1: [u8] from "assets/random.dat" with zstd if always);
flate!(pub static DATA2: [u8] from "assets/random.dat" with deflate if less_than_original);
flate!(pub static DATA3: [u8] from "assets/random.dat" with deflate if compression_ratio_more_than 0);

flate!(pub static DATA4: [u8] from "assets/random.dat" if always);
flate!(pub static DATA5: [u8] from "assets/random.dat" if less_than_original);
flate!(pub static DATA6: [u8] from "assets/random.dat" if compression_ratio_more_than 0);

flate!(pub static DATA7: str from "assets/chinese.txt" with zstd if always);
flate!(pub static DATA8: str from "assets/chinese.txt" with deflate if less_than_original);
flate!(pub static DATA9: str from "assets/chinese.txt" with deflate if compression_ratio_more_than 0);

flate!(pub static DATA10: str from "assets/chinese.txt" if always);
flate!(pub static DATA11: str from "assets/chinese.txt" if less_than_original);
flate!(pub static DATA12: str from "assets/chinese.txt" if compression_ratio_more_than 0);
