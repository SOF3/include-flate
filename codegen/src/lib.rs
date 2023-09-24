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

#![cfg_attr(not(feature = "stable"), feature(proc_macro_span))]

extern crate proc_macro;

use std::fs::File;
use std::path::PathBuf;
use std::str::from_utf8;

use libflate::deflate::Encoder;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{Error, ExprTuple, LitByteStr, LitStr, Result};

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
#[cfg_attr(feature = "stable", proc_macro_hack::proc_macro_hack)]
#[cfg_attr(not(feature = "stable"), proc_macro)]
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
#[cfg_attr(feature = "stable", proc_macro_hack::proc_macro_hack)]
#[cfg_attr(not(feature = "stable"), proc_macro)]
pub fn deflate_utf8_file(ts: TokenStream) -> TokenStream {
    match inner(ts, true) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn inner(ts: TokenStream, utf8: bool) -> Result<impl Into<TokenStream>> {
    if let Ok(t) = syn::parse::<ExprTuple>(ts.clone()) {
        if t.elems.len() != 2 {
            panic!("expected a tuple of size 2 only");
        }
        let (lit, base) = (
            t.elems.first().unwrap().into_token_stream(),
            t.elems.last().unwrap().into_token_stream(),
        );
        let (lit, base) = (
            syn::parse::<LitStr>(lit.into())?,
            syn::parse::<LitStr>(base.into())?,
        );
        let key = quote!(#base).to_string();
        compress_file_as_tokenstream(PathBuf::from(lit.value()), &key[1..key.len() - 1], utf8)
    } else if let Ok(lit) = syn::parse::<LitStr>(ts) {
        compress_file_as_tokenstream(PathBuf::from(lit.value()), "CARGO_MANIFEST_DIR", utf8)
    } else {
        panic!("invalid pattern")
    }
}

fn compress_file_as_tokenstream(
    path: PathBuf,
    key: &str,
    utf8: bool,
) -> Result<impl Into<TokenStream>> {
    fn emap<E: std::fmt::Display>(error: E) -> Error {
        Error::new(Span::call_site(), error)
    }

    if path.is_absolute() {
        Err(emap("absolute paths are not supported"))?;
    }

    let dir = PathBuf::from(std::env::var(key).unwrap());
    let target: PathBuf = dir.join(path);

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

    let bytes = LitByteStr::new(&bytes, Span::call_site());
    let result = quote!(#bytes);

    Ok(result)
}
