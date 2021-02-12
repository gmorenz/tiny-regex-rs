#!/bin/sh
set -e

# Bench Rust

if [ ! -d tiny-regex-c ]; then
    git clone https://github.com/gmorenz/tiny-regex-c --branch rust_build
fi
cargo build --example tiny_regex_rs --release
cp target/release/examples/libtiny_regex_rs.a tiny-regex-c
cd tiny-regex-c
git checkout rust_build
make rust_all
echo "Rust time" > ../timings
/usr/bin/time --output=../timings --append make test

# Bench C

git checkout master
make all
echo "C time" >> ../timings
/usr/bin/time --output=../timings --append make test