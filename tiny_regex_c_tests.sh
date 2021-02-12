#!/bin/sh
set -ex
if [ ! -d tiny-regex-c ]; then
    git clone https://github.com/gmorenz/tiny-regex-c --branch rust_build
fi
cargo build --example tiny_regex_rs --release
cp target/release/examples/libtiny_regex_rs.a tiny-regex-c
cd tiny-regex-c
make test
