#!/bin/sh
cargo clippy --no-default-features --features left,serial --target thumbv6m-none-eabi
cargo clippy --no-default-features --features left,hiddev --target thumbv6m-none-eabi
cargo clippy --no-default-features --features right,serial --target thumbv6m-none-eabi
cargo clippy --no-default-features --features right --target thumbv6m-none-eabi
