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

use libflate::deflate;

#[doc(hidden)]
pub use include_flate_codegen as codegen;
#[doc(hidden)]
pub use lazy_static::lazy_static;

#[macro_export]
macro_rules! flate {
    ($view:vis static $name:ident: [u8] from $path:literal) => {
        $crate::lazy_static! {
            $view static ref $name: ::std::vec::Vec<u8> = $crate::decode($crate::codegen::deflate_file!($path));
        }
    };
    ($view:vis static $name:ident: str from $path:literal) => {
        $crate::lazy_static! {
            $view static ref $name: ::std::string::String = $crate::decode_string($crate::codegen::deflate_utf8_file!($path));
        }
    };
}

#[doc(hidden)]
pub fn decode(bytes: &[u8]) -> Vec<u8> {
    use std::io::{Cursor, Read};

    let mut dec = deflate::Decoder::new(Cursor::new(bytes));
    let mut ret = Vec::new();
    dec.read_to_end(&mut ret).expect("Compiled DEFLATE buffer was corrupted");
    ret
}

#[doc(hidden)]
pub fn decode_string(bytes: &[u8]) -> String {
    // We should have checked for utf8 correctness in encode_utf8_file!
    String::from_utf8(decode(bytes)).expect("flate_str has malformed UTF-8 despite checked at compile time")
}
