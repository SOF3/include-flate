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
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::str::from_utf8;

use include_flate_compress::{apply_compression, apply_decompression, CompressionMethod};

pub fn get_file_path<P: AsRef<Path>>(relative_from: Option<&Path>, path: P) -> PathBuf {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let default_base_path = Path::new(&cargo_manifest_dir);
    let base_path = relative_from.unwrap_or_else(|| default_base_path);
    base_path.join("assets").join(path)
}

pub fn read_file<P: AsRef<Path>>(name: P) -> Vec<u8> {
    let path = get_file_path(None, name);
    let mut vec = Vec::<u8>::new();
    std::io::copy(&mut File::open(path).unwrap(), &mut vec).unwrap();
    vec
}

pub fn verify_compression<P: AsRef<Path>>(name: P, data: &[u8], method: CompressionMethod) {
    let path = get_file_path(None, &name);
    let mut file = File::open(&path).unwrap();
    let mut file_buffer = Vec::new();
    file.read_to_end(&mut file_buffer).unwrap();
    let mut source = std::io::Cursor::new(file_buffer);
    let mut compressed_buffer = Vec::new();
    {
        let mut compressed_cursor = std::io::Cursor::new(&mut compressed_buffer);
        apply_compression(&mut source, &mut compressed_cursor, method).unwrap();
        compressed_cursor.seek(SeekFrom::Start(0)).unwrap(); // Reset cursor position
    }
    assert_ne!(compressed_buffer.as_slice(), data);
    let mut decompressed_buffer = Vec::new();
    {
        let mut compressed_cursor = std::io::Cursor::new(&mut compressed_buffer);
        let mut decompressed_cursor = std::io::Cursor::new(&mut decompressed_buffer);
        apply_decompression(&mut compressed_cursor, &mut decompressed_cursor, method).unwrap();
        decompressed_cursor.seek(SeekFrom::Start(0)).unwrap(); // Reset cursor position
    }
    assert_ne!(compressed_buffer.as_slice(), decompressed_buffer.as_slice());
}

pub fn verify<P: AsRef<Path>>(name: P, data: &[u8]) {
    verify_compression(&name, data, CompressionMethod::Deflate);
    verify_compression(&name, data, CompressionMethod::Zstd);
    assert_eq!(read_file(&name), data);
}

pub fn verify_str(name: &str, data: &str) {
    // But the file should not have compiled in the first place
    assert_eq!(
        from_utf8(&read_file(name)).expect("File is not encoded"),
        data
    );
}
