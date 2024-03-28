#!/bin/sh
if [[ "$1" == "h" ]]
then
  cargo b -r --no-default-features --features hiddev --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/release/rp2040-kbd
else
  cargo b -r --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/release/rp2040-kbd
fi
