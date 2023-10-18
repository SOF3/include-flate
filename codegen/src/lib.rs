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

#[cfg(all(feature = "deflate", feature = "zstd"))]
compile_error!("You cannot enable both `deflate` and `zstd` at the same time.");
#[cfg(not(any(feature = "deflate", feature = "zstd")))]
compile_error!("You must enable either the `deflate` or `zstd` feature.");

extern crate proc_macro;

use std::io::Write;
use std::path::PathBuf;
use std::str::from_utf8;
use std::{fs::File, os::windows::prelude::MetadataExt};

#[cfg(feature = "deflate")]
use libflate::deflate::Encoder;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Error, LitByteStr, LitStr, Result};

/// `deflate_file!("file")` is equivalent to `include_bytes!("file.gz")`.
///
/// # Parameters
/// This macro accepts exactly one literal parameter that refers to a path relative to
/// `CARGO_MANIFEST_DIR`. Absolute paths are not supported.
///
/// Note that **this is distinct from the behaviour of the builtin `include_bytes!`/`include_str!` macros** &mdash;
/// `includle_bytes!`/`include_str!` paths are relative to the current source file, while `deflate_file!` paths are relative to
/// `CARGO_MANIFEST_DIR`.
///
/// # Returns
/// This macro expands to a `b"byte string"` literal that contains the deflated form of the file.
///
/// # Compile errors
/// - If the argument is not a single literal
/// - If the referenced file does not exist or is not readable
#[proc_macro]
pub fn deflate_file(ts: TokenStream) -> TokenStream {
    match inner(ts, false) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// This macro is identical to `deflate_file!()`, except it additionally performs UTF-8 validation.
///
/// # Compile errors
/// - The compile errors in `deflate_file!`
/// - If the file contents are not all valid UTF-8
#[proc_macro]
pub fn deflate_utf8_file(ts: TokenStream) -> TokenStream {
    match inner(ts, true) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn inner(ts: TokenStream, utf8: bool) -> Result<impl Into<TokenStream>> {
    fn emap<E: std::fmt::Display>(error: E) -> Error {
        Error::new(Span::call_site(), error)
    }

    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let lit = syn::parse::<LitStr>(ts)?;
    let path = PathBuf::from(lit.value());

    if path.is_absolute() {
        Err(emap("absolute paths are not supported"))?;
    }

    let target = dir.join(path);

    let mut file = File::open(target).map_err(emap)?;
    let before = file.metadata().unwrap().len();
    println!("before size: {:#X}", before);

    let mut vec = Vec::<u8>::new();
    if utf8 {
        std::io::copy(&mut file, &mut vec).map_err(emap)?;
        from_utf8(&vec).map_err(emap)?;
    }

    #[allow(unused_assignments)]
    let mut bytes = Vec::new();

    #[cfg(feature = "zstd")]
    {
        let mut encoder = zstd::stream::Encoder::new(Vec::<u8>::new(), 0).unwrap();
        if utf8 {
            encoder.write_all(&vec).map_err(emap)?;
        } else {
            // no need to store the raw buffer; let's avoid storing two buffers
            std::io::copy(&mut file, &mut encoder).map_err(emap)?;
        }
        bytes = encoder.finish().unwrap();
    }

    #[cfg(feature = "deflate")]
    {
        let mut encoder = Encoder::new(Vec::<u8>::new());
        if utf8 {
            encoder.write_all(&vec).map_err(emap)?;
        } else {
            // no need to store the raw buffer; let's avoid storing two buffers
            std::io::copy(&mut file, &mut encoder).map_err(emap)?;
        }
        bytes = encoder.finish().into_result().map_err(emap)?;
    }

    let bytes = LitByteStr::new(&bytes, Span::call_site());
    let result = quote!(#bytes);

    Ok(result)
}
