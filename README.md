# include-flate
[![Travis-CI](https://travis-ci.com/SOF3/include-flate.svg?branch=master)](https://travis-ci.com/SOF3/include-flate)
[![crates.io](https://img.shields.io/crates/dv/include-flate.svg)](https://docs.rs/include-flate)

A variant of `include_bytes!`/`include_str!` with compile-time deflation and runtime lazy inflation.

## Why?
`include_bytes!`/`include_str!` are great for embedding resources into an executable/library
without involving the complex logistics of maintaining an assets manager.
However, they are copied as-is into the artifact, leading to unnecessarily large binary size.
This library automatically compresses the resources and lazily decompresses them at runtime,
allowing smaller binary sizes.

Nevertheless, this inevitably leads to wasting RAM to store both the compressed and decompressed data,
which might be undesirable if the data are too large.
An actual installer is still required if the binary involves too many resources that do not need to be kept in RAM all time.

## Warning
This library compresses included data independently.
It is usually more effective to compress the whole output binary together (e.g. distributing `.exe.gz` )
than to compress independently.
In addition, compression algorithms usually produce smaller artifacts by processing the raw input together
than by processing already-compressed output.
`#[cfg_attr]` might come handy for conditionally using compression or direct data inclusion.
