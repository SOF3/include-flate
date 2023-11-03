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

extern crate proc_macro;

use std::fs::{self, File};
use std::io::{Read, Seek};
use std::path::PathBuf;
use std::str::{from_utf8, FromStr};

use include_flate_compress::{apply_compression, CompressionMethod};
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{emit_warning, proc_macro_error};
use quote::quote;
use syn::{Error, LitByteStr};

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
#[proc_macro_error]
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
#[proc_macro_error]
pub fn deflate_utf8_file(ts: TokenStream) -> TokenStream {
    match inner(ts, true) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// An arguments expected provided by the proc-macro.
///
/// ```ignore
/// flate!(pub static DATA: [u8] from "assets/009f.dat"); // default, DEFLATE
/// flate!(pub static DATA: [u8] from "assets/009f.dat" with zstd); // Use Zstd for this file spcifically
/// flate!(pub static DATA: [u8] from "assets/009f.dat" with deflate); // Explicitly use DEFLATE.
/// ```
struct FlateArgs {
    path: syn::LitStr,
    algorithm: Option<CompressionMethodTy>,
}

impl syn::parse::Parse for FlateArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;

        let algorithm = if input.is_empty() {
            None
        } else {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::deflate) {
                input.parse::<kw::deflate>()?;
                Some(CompressionMethodTy(CompressionMethod::Deflate))
            } else if lookahead.peek(kw::zstd) {
                input.parse::<kw::zstd>()?;
                Some(CompressionMethodTy(CompressionMethod::Zstd))
            } else {
                return Err(lookahead.error());
            }
        };

        Ok(Self { path, algorithm })
    }
}

mod kw {
    syn::custom_keyword!(deflate);
    syn::custom_keyword!(zstd);
}

#[derive(Debug)]
struct CompressionMethodTy(CompressionMethod);

fn compression_ratio(original_size: u64, compressed_size: u64) -> f64 {
    (compressed_size as f64 / original_size as f64) * 100.0
}

fn inner(ts: TokenStream, utf8: bool) -> syn::Result<impl Into<TokenStream>> {
    fn emap<E: std::fmt::Display>(error: E) -> Error {
        Error::new(Span::call_site(), error)
    }

    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(emap)?);

    let args: FlateArgs = syn::parse2::<FlateArgs>(ts.to_owned().into())?;
    let path = PathBuf::from_str(&args.path.value()).map_err(emap)?;
    let algo = args
        .algorithm
        .unwrap_or(CompressionMethodTy(CompressionMethod::Deflate));

    if path.is_absolute() {
        Err(emap("absolute paths are not supported"))?;
    }

    let target = dir.join(&path);

    let mut file = File::open(&target).map_err(emap)?;

    let mut vec = Vec::<u8>::new();
    if utf8 {
        std::io::copy(&mut file, &mut vec).map_err(emap)?;
        from_utf8(&vec).map_err(emap)?;
    }

    let mut compressed_buffer = Vec::<u8>::new();

    {
        let mut compressed_cursor = std::io::Cursor::new(&mut compressed_buffer);
        let mut source: Box<dyn Read> = if utf8 {
            Box::new(std::io::Cursor::new(vec))
        } else {
            file.seek(std::io::SeekFrom::Start(0)).map_err(emap)?;
            Box::new(&file)
        };

        apply_compression(&mut source, &mut compressed_cursor, algo.0).map_err(emap)?;
    }

    let bytes = LitByteStr::new(&compressed_buffer, Span::call_site());
    let result = quote!(#bytes);

    #[cfg(not(feature = "no-compression-warnings"))]
    {
        let compression_ratio = compression_ratio(
            fs::metadata(&target).map_err(emap)?.len(),
            compressed_buffer.len() as u64,
        );

        if compression_ratio < 10.0f64 {
            emit_warning!(
            &args.path,
            "Detected low compression ratio ({:.2}%) for file {:?} with `{:?}`. Consider using other compression methods.",
            compression_ratio,
            path.display(),
            algo.0,
        );
        }
    }

    Ok(result)
}
