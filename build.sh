#!/bin/sh
if [[ "$1" == "l" ]]
then
  if [[ "$2" == "d" ]]
  then
    cargo b --profile lto --no-default-features --features left,serial --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd && echo "Left serial $(ls -lah target/thumbv6m-none-eabi/lto/rp2040-kbd.uf2)"
  else
    cargo b --profile lto --no-default-features --features left,hiddev --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd && echo "Left hiddev $(ls -lah target/thumbv6m-none-eabi/lto/rp2040-kbd.uf2)"
  fi
else
    if [[ "$2" == "d" ]]
    then
      cargo b --profile lto --no-default-features --features right,serial --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd && echo "Right serial $(ls -lah target/thumbv6m-none-eabi/lto/rp2040-kbd.uf2)"
    else
      cargo b --profile lto --no-default-features --features right --target thumbv6m-none-eabi && elf2uf2-rs target/thumbv6m-none-eabi/lto/rp2040-kbd && echo "Right $(ls -lah target/thumbv6m-none-eabi/lto/rp2040-kbd.uf2)"
    fi
fi
