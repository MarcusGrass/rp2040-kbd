#!/bin/sh
if [[ "$1" == "l" ]]
then
  if [[ "$2" == "d" ]]
  then
    cargo b --profile lto --no-default-features --features left,serial --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
  else
    cargo b --profile lto --no-default-features --features left,hiddev --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
  fi
else
    if [[ "$2" == "d" ]]
    then
      cargo b --profile lto --features right,serial --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
    else
      cargo b --profile lto --features right --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd
    fi
fi
