# rust-srcprof

This is reimplementation of source code line count profile I wrote long time ago in Python.

As it is implemented in Rust, it should be much faster than Python version, especially with
large repository.

Of course, number of lines is a stupid metric to evaluate a code base in any standard,
but still it gives some amount of insight if you have no knowledge at all about it.

## Prerequisites

* Cargo 1.56.0

## How to run

   cargo run [options] <path>

Full list of options can be obtained by `cargo run -- --help`.
