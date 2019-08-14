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

#![feature(proc_macro_span)]

extern crate proc_macro;

use std::fs::File;
use std::path::PathBuf;
use std::str::from_utf8;

use libflate::deflate::Encoder;
use proc_macro::{TokenStream};
use proc_macro2::{Literal, Span};
use quote::quote;
use syn::{Error, LitStr, Result};

/// `deflate_file!("file")` is equivalent to `include_bytes!("file.gz")`.
///
/// # Parameters
/// This macro accepts exactly one literal parameter that refers to a path, either absolute or relative to `CARGO_MANIFEST_DIR`.
///
/// Note that **this is distinct from the behaviour of the builtin `include_bytes!`/`include_str!` macros** &mdash;
/// `includle_bytes!`/`include_str!` paths are relative to the current source file, while `deflate_file!` paths are relative to
/// `CARGO_MANIFEST_DIR`.
///
/// # Returns
/// This macro expands to a `&[u8]` literal that contains the deflated form of the file.
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
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let lit = syn::parse::<LitStr>(ts)?;
    let path = PathBuf::from(lit.value());

    let target = if path.is_relative() {
        dir.join(path)
    } else {
        path
    };

    fn emap<E: std::fmt::Display>(error: E) -> Error {
        Error::new(Span::call_site(), error)
    }

    let mut file = File::open(target).map_err(emap)?;

    let mut encoder = Encoder::new(Vec::<u8>::new());
    if utf8 {
        use std::io::Write;

        let mut vec = Vec::<u8>::new();
        std::io::copy(&mut file, &mut vec).map_err(emap)?;
        from_utf8(&vec).map_err(emap)?;
        encoder.write_all(&vec).map_err(emap)?;
    } else {
        // no need to store the raw buffer; let's avoid storing two buffers
        std::io::copy(&mut file, &mut encoder).map_err(emap)?;
    }
    let bytes = encoder.finish().into_result().map_err(emap)?;

    let bytes = bytes.into_iter().map(|byte| Literal::u8_suffixed(byte));
    let result = quote!(&[#(#bytes),*]);

    Ok(result)
}
