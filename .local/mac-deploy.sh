### Mac doesn't seem to handle the pseudo FAT-16 drive particularly well
#!/bin/bash
sudo diskutil unmount force /dev/disk4s1
sudo mkdir -p /Volumes/rp2040
sudo mount_msdos -u $(id -u) -g $(id -g) /dev/disk4s1 /Volumes/rp2040
cp target/thumbv6m-none-eabi/lto/rp2040-kbd.uf2 /Volumes/rp2040/