#!/bin/bash

source_dir="$(dirname "${BASH_SOURCE[0]}")"
pushd "$source_dir"

mkdir -p out
cargo build
cargo run -q -- schema > out/schema.json
cargo run -q -- demo > out/demo.yml
cargo run -q -- md requirements.yml > out/requirements.md
cargo run -q -- html requirements.yml > out/requirements.html
cargo run -q -- check requirements.yml test_result.txt > out/text_result.md
