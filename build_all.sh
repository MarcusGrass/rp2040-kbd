#!/bin/sh
set -ex
cargo b --profile lto --no-default-features --features left,hiddev --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
cargo b --profile lto --no-default-features --features left,serial --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
cargo b --profile lto --no-default-features --features right --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
cargo b --profile lto --no-default-features --features right,serial --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
