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

use std::fs::File;
use std::path::PathBuf;
use std::str::from_utf8;

pub fn read_file(name: &str) -> Vec<u8> {
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets").join(name);
    let mut vec = Vec::<u8>::new();
    std::io::copy(&mut File::open(path).unwrap(), &mut vec).unwrap();
    vec
}

pub fn verify(name: &str, data: &[u8]) {
    assert_eq!(read_file(name), data);
}

pub fn verify_str(name: &str, data: &str) {
    // But the file should not have compiled in the first place
    assert_eq!(from_utf8(&read_file(name)).expect("File is not encoded"), data);
}
