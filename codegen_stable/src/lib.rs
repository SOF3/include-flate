use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
pub use include_flate_codegen::deflate_file;

#[proc_macro_hack]
pub use include_flate_codegen::deflate_utf8_file;