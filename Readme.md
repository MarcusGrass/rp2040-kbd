# Keyboard firmware for the rp2040 on a lily58

There are probably bugs here, don't use this!

A more in-depth description of this repo is [here](https://marcusgrass.github.io/rust-kbd.html), 
or [here if github pages is down](https://github.com/MarcusGrass/marcusgrass.github.io/blob/main/pages/projects/RustKbd.md). 

## Build and useful commands

When the rp2040 goes into boot-mode it'll 
show up as a disk.  

### Build and flash right side as hiddev

Keyboard put into boot mode, shows up as /dev/sdb:

1. `.local/build.sh r h`
2. `mount /dev/sdb1 /mnt/rp2040 && cp code/rust/rp2040-kbd/target/thumbv6m-none-eabi/lto/rp2040-kbd.uf2 /mnt/rp2040 && umount /mnt/rp2040`

### Debug through serial

Build left side (for example) in debug. 

Creates a picocom connection to interface with the kbd.  

1. `.local/build.sh l d`
2. `mount /dev/sdb1 /mnt/rp2040 && cp code/rust/rp2040-kbd/target/thumbv6m-none-eabi/lto/rp2040-kbd.uf2 /mnt/rp2040 && umount /mnt/rp2040`
3. `picocom -b 115200 -l /dev/ttyACM0`




## License

[GPLV3, see here](LICENSE)
