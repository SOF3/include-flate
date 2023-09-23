#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;

#[cfg_attr(feature = "stable", proc_macro_hack::proc_macro_hack)]
pub use include_flate_codegen::deflate_file;

#[cfg_attr(feature = "stable", proc_macro_hack::proc_macro_hack)]
pub use include_flate_codegen::deflate_utf8_file;

#[cfg(feature = "std")]
pub type String = ::std::string::String;

#[cfg(not(feature = "std"))]
pub type String = ::alloc::string::String;

#[cfg(feature = "std")]
pub type Vec<T> = ::std::vec::Vec<T>;

#[cfg(not(feature = "std"))]
pub type Vec<T> = ::alloc::vec::Vec<T>;
