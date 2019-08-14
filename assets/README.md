# assets
This directory contains some binary and text files used as test cases

## 009f.dat
`9f` is not a valid UTF-8 codepoint. This is intended to make the UTF-8 validator fail.

## ascii-printable.txt
All characters in this file are printable ASCII characters. ASCII is a subset of UTF-8.

## ascii-control.txt
This file contains all ASCII characters, including printable and control characters. ASCII is a subset of UTF-8.

## chinese.txt
This file contains 1323 Chinese characters encoded in UTF-8.

## emoji.txt
This file contains 64 4-byte emojis encoded in UTF-8. (We are not specifically working on glyphs here, so no need to waste time defining "character" precisely)

## zero.dat
This file contains 1048576 (1 MB) null bytes. This consistency is useful for benchmarking compression.

## ff.dat
This file contains 1048576 (1 MB) 0xFF bytes. This consistency is useful for benchmarking compression.

## base64.txt
This file contains 1048576 (1 MB) bytes, repeating the base64 charset for 16384 times.

## random.dat
This file contains 1048576 (1 MB) random bytes. This entropy is useful for benchmarking compression.
