# rust-srcprof

This is reimplementation of source code line count profile I wrote long time ago in Python.

As it is implemented in Rust, it should be much faster than Python version, especially with
large repository.

## Prerequisites

* Cargo 1.56.0

## How to run

   cargo run [options] <path>

Full list of options can be obtained by `cargo run -- --help`.
