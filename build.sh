#!/bin/sh
if [[ "$1" == "h" ]]
then
  cargo b --profile lto --no-default-features --features hiddev --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
else
  cargo b --profile lto --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
fi
