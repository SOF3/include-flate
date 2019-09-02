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

//! A variant of `include_bytes!`/`include_str!` with compile-time deflation and runtime lazy inflation.
//!
//! ## Why?
//! `include_bytes!`/`include_str!` are great for embedding resources into an executable/library
//! without involving the complex logistics of maintaining an assets manager.
//! However, they are copied as-is into the artifact, leading to unnecessarily large binary size.
//! This library automatically compresses the resources and lazily decompresses them at runtime,
//! allowing smaller binary sizes.
//!
//! Nevertheless, this inevitably leads to wasting RAM to store both the compressed and decompressed data,
//! which might be undesirable if the data are too large.
//! An actual installer is still required if the binary involves too many resources that do not need to be kept in RAM all time.

use libflate::deflate;

/// The low-level macros used by this crate.
pub use include_flate_codegen_exports as codegen;
#[doc(hidden)]
pub use lazy_static::lazy_static;

/// This macro is like [`include_bytes!`][1] or [`include_str!`][2], but compresses at compile time
/// and lazily decompresses at runtime.
///
/// # Parameters
/// The macro can be used like this:
/// ```ignore
/// flate!($meta $vis static $name: $type from $file);
/// ```
///
/// - `$meta` is zero or more `#[...]` attributes that can be applied on the static parameters of
/// `lazy_static`. For the actual semantics of the meta attributes, please refer to
/// [`lazy_static`][3] documentation.
/// - `$vis` is a visibility modifier (e.g. `pub`, `pub(crate)`) or empty.
/// - `$name` is the name of the static variable..
/// - `$type` can be either `[u8]` or `str`. However, the actual type created would dereference
/// into `Vec<u8>` and `String` (although they are `AsRef<[u8]>` and `AsRef<str>`) respectively.
/// - `$file` is either an absolute path or a path relative to the current
/// [`CARGO_MANIFEST_DIR`][4]. Note that **this is distinct from the behaviour of the builtin
/// `include_bytes!`/`include_str!` macros** &mdash; `includle_bytes!`/`include_str!` paths are
/// relative to the current source file, while `flate!` paths are relative to `CARGO_MANIFEST_DIR`.
///
/// # Returns
/// The macro expands to a [`lazy_static`][3] call, which lazily inflates the compressed bytes.
///
/// # Compile errors
/// - If the input format is incorrect
/// - If the referenced file does not exist or is not readable
/// - If `$type` is `str` but the file is not fully valid UTF-8
///
/// # Algorithm
/// Compression and decompression use the DEFLATE algorithm from [`libflate`][5].
///
/// # Examples
/// Below are some basic examples. For actual compiled examples, see the [`tests`][6] directory.
///
/// ```ignore
/// // This declares a `static VAR_NAME: impl Deref<Vec<u8>>`
/// flate!(static VAR_NAME: [u8] from "binary-file.dat");
///
/// // This declares a `static VAR_NAME: impl Deref<String>`
/// flate!(static VAR_NAME: str from "text-file.txt");
///
/// // Visibility modifiers can be added in the front
/// flate!(pub static VAR_NAME: str from "public-file.txt");
///
/// // Meta attributes can also be added
/// flate!(#[allow(unused)]
///        #[doc = "Example const"]
///        pub static VAR_NAME: str from "file.txt");
/// ```
///
///   [1]: https://doc.rust-lang.org/std/macro.include_bytes.html
///   [2]: https://doc.rust-lang.org/std/macro.include_str.html
///   [3]: https://docs.rs/lazy_static/1.3.0/lazy_static/
///   [4]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
///   [5]: https://docs.rs/libflate/0.1.26/libflate/
///   [6]: https://github.com/SOF3/include-flate/tree/master/tests
#[macro_export]
macro_rules! flate {
    ($(#[$meta:meta])*
        $(pub $(($($vis:tt)+))?)? static $name:ident: [u8] from $path:literal) => {
        $crate::lazy_static! {
            $(#[$meta])*
            $(pub $(($($vis)+))?)? static ref $name: ::std::vec::Vec<u8> = $crate::decode($crate::codegen::deflate_file!($path));
        }
    };
    ($(#[$meta:meta])*
        $(pub $(($($vis:tt)+))?)? static $name:ident: str from $path:literal) => {
        $crate::lazy_static! {
            $(#[$meta])*
            $(pub $(($($vis)+))?)? static ref $name: ::std::string::String = $crate::decode_string($crate::codegen::deflate_utf8_file!($path));
        }
    };
}

#[doc(hidden)]
pub fn decode(bytes: &[u8]) -> Vec<u8> {
    use std::io::{Cursor, Read};

    let mut dec = deflate::Decoder::new(Cursor::new(bytes));
    let mut ret = Vec::new();
    dec.read_to_end(&mut ret)
        .expect("Compiled DEFLATE buffer was corrupted");
    ret
}

#[doc(hidden)]
pub fn decode_string(bytes: &[u8]) -> String {
    // We should have checked for utf8 correctness in encode_utf8_file!
    String::from_utf8(decode(bytes))
        .expect("flate_str has malformed UTF-8 despite checked at compile time")
}
