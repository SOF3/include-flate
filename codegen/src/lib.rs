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

use include_flate_compress::{apply_compression, compression_ratio, CompressionMethod};
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{emit_warning, proc_macro_error};
use quote::quote;
use syn::{Error, LitByteStr, LitInt, Token};

/// This macro evaluates to `true` if the file should be compressed, `false` otherwise, at compile time.
/// Useful for conditional compilation without any efforts to the runtime.
///
/// Please note that unlike the macro names suggest, this macro does **not** actually compress the file.
///
/// # Parameters
/// This macro accepts custom compression methods and threshold conditions.
///
/// # Returns
/// This macro expands to a `bool` literal that indicates whether the file should be compressed.
/// If no condition is specified, this macro always returns `true`.
#[proc_macro]
#[proc_macro_error]
pub fn deflate_if(ts: TokenStream) -> TokenStream {
    match deflate_if_inner(ts, false) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// This macro is identical to `deflate_if!()`, except it additionally performs UTF-8 validation.
/// See `deflate_if!` for more details.
#[proc_macro]
#[proc_macro_error]
pub fn deflate_utf8_if(ts: TokenStream) -> TokenStream {
    match deflate_if_inner(ts, true) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

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
    match deflate_inner(ts, false) {
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
    match deflate_inner(ts, true) {
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
///
/// flate!(pub static DATA: [u8] from "assets/009f.dat" if always); // Always compress regardless of compression ratio.
/// flate!(pub static DATA: [u8] from "assets/009f.dat" if less_than_original); // Compress only if the compressed size is smaller than the original size.
/// flate!(pub static DATA: [u8] from "assets/009f.dat" if compression_ratio_more_than 10%); // Compress only if the compression ratio is higher than 10%.
/// ```
struct FlateArgs {
    path: syn::LitStr,
    algorithm: Option<CompressionMethodTy>,
    threshold: Option<ThresholdCondition>,
}

impl syn::parse::Parse for FlateArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;

        let mut algorithm = None;
        let mut threshold = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::deflate) || lookahead.peek(kw::zstd) {
                algorithm = if lookahead.peek(kw::deflate) {
                    input.parse::<kw::deflate>()?;
                    Some(CompressionMethodTy(CompressionMethod::Deflate))
                } else {
                    input.parse::<kw::zstd>()?;
                    Some(CompressionMethodTy(CompressionMethod::Zstd))
                };
            } else if lookahead.peek(kw::always)
                || lookahead.peek(kw::less_than_original)
                || (lookahead.peek(kw::compression_ratio_more_than)
                    && input.peek2(syn::LitInt)
                    && input.peek3(Token![%]))
            {
                threshold = Some(input.parse()?);
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(Self {
            path,
            algorithm,
            threshold,
        })
    }
}

/// A threshold condition for compression.
enum ThresholdCondition {
    /// Always compress regardless of compression ratio.
    /// This is the default behaviour.
    Always,
    /// Compress only if the compressed size is smaller than the original size.
    LessThanOriginal,
    /// Compress only if the compression ratio is higher than the given threshold.
    CompressionRatioMoreThan(u64),
}

impl syn::parse::Parse for ThresholdCondition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::always) {
            input.parse::<kw::always>()?;
            Ok(Self::Always)
        } else if lookahead.peek(kw::less_than_original) {
            input.parse::<kw::less_than_original>()?;
            Ok(Self::LessThanOriginal)
        } else if lookahead.peek(kw::compression_ratio_more_than) {
            input.parse::<kw::compression_ratio_more_than>()?;
            let lit: LitInt = input.parse()?;
            input.parse::<Token![%]>()?;
            Ok(Self::CompressionRatioMoreThan(lit.base10_parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl Into<u64> for ThresholdCondition {
    fn into(self) -> u64 {
        match self {
            Self::Always => 0,
            Self::LessThanOriginal => 100,
            Self::CompressionRatioMoreThan(threshold) => threshold,
        }
    }
}

/// Custom keywords for the proc-macro.
mod kw {
    // `deflate` is a keyword that indicates that the file should be compressed with DEFLATE.
    syn::custom_keyword!(deflate);
    // `zstd` is a keyword that indicates that the file should be compressed with Zstd.
    syn::custom_keyword!(zstd);

    // `always` is a keyword that indicates that the file should always be compressed.
    syn::custom_keyword!(always);
    // `less_than_original` is a keyword that indicates that the file should be compressed only if the compressed size is larger than the original size.
    syn::custom_keyword!(less_than_original);
    // `compression_ratio_more_than` is a keyword that indicates that the file should be compressed only if the compression ratio is less than the given threshold.
    // For example, `compression_ratio_more_than 10%` means that the file should be compressed only if the compressed size is less than 10% of the original size.
    syn::custom_keyword!(compression_ratio_more_than);
}

#[derive(Debug)]
struct CompressionMethodTy(CompressionMethod);

fn emap<E: std::fmt::Display>(error: E) -> Error {
    Error::new(Span::call_site(), error)
}

fn deflate_if_inner(ts: TokenStream, utf8: bool) -> syn::Result<impl Into<TokenStream>> {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(emap)?);

    let args = syn::parse2::<FlateArgs>(ts.to_owned().into())?;
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
            Box::new(std::io::Cursor::new(&vec))
        } else {
            file.seek(std::io::SeekFrom::Start(0)).map_err(emap)?;
            Box::new(&file)
        };

        apply_compression(&mut source, &mut compressed_cursor, algo.0).map_err(emap)?;
    }

    let compression_ratio = compression_ratio(
        fs::metadata(&target).map_err(emap)?.len(),
        compressed_buffer.len() as u64,
    );

    // returns `true` if the file should be compressed, `false` otherwise.
    match args.threshold {
        Some(ThresholdCondition::Always) => Ok(quote!(true)),
        Some(ThresholdCondition::LessThanOriginal) => {
            if compressed_buffer.len() > vec.len() {
                Ok(quote!(false))
            } else {
                Ok(quote!(true))
            }
        }
        Some(ThresholdCondition::CompressionRatioMoreThan(threshold)) => {
            if compression_ratio > threshold as f64 {
                Ok(quote!(false))
            } else {
                Ok(quote!(true))
            }
        }
        _ => Ok(quote!(true)),
    }
}

fn deflate_inner(ts: TokenStream, utf8: bool) -> syn::Result<impl Into<TokenStream>> {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(emap)?);

    let args = syn::parse2::<FlateArgs>(ts.to_owned().into())?;
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
            Box::new(std::io::Cursor::new(&vec))
        } else {
            file.seek(std::io::SeekFrom::Start(0)).map_err(emap)?;
            Box::new(&file)
        };

        apply_compression(&mut source, &mut compressed_cursor, algo.0).map_err(emap)?;
    }

    let bytes = LitByteStr::new(&compressed_buffer, Span::call_site());
    let result = quote!(#bytes);

    let compression_ratio = compression_ratio(
        fs::metadata(&target).map_err(emap)?.len(),
        compressed_buffer.len() as u64,
    );

    // Default to 10% threshold
    let threshold: u64 = args.threshold.map_or(10, |cond| cond.into());

    #[cfg(not(feature = "no-compression-warnings"))]
    {
        if compression_ratio < threshold as f64 {
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
