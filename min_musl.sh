#!/bin/sh
RUSTFLAGS="-Zunstable-options -Zlocation-detail=none -Zfmt-debug=none -Cpanic=immediate-abort" cargo +nightly $@ \
  -Z build-std=std,panic_abort \
  -Z build-std-features=optimize_for_size \
  --target x86_64-unknown-linux-musl \
  --profile=min_size
