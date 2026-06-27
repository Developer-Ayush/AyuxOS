#!/bin/bash

KERNEL_IMAGE=$1
INITRAMFS=$2

if [ -z "$KERNEL_IMAGE" ] || [ -z "$INITRAMFS" ]; then
    echo "Usage: $0 <kernel_image> <initramfs>"
    exit 1
fi

qemu-system-x86_64 \
    -kernel "$KERNEL_IMAGE" \
    -initrd "$INITRAMFS" \
    -append "console=ttyS0 quiet" \
    -nographic \
    -m 512M \
    -no-reboot
