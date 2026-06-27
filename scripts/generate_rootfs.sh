#!/bin/bash
set -e

BUILD_DIR=$1
REPO_ROOT=$2

ROOTFS_DIR="${BUILD_DIR}/rootfs"

echo "Assembling rootfs in ${ROOTFS_DIR}..."

rm -rf "${ROOTFS_DIR}"
mkdir -p "${ROOTFS_DIR}"
cd "${ROOTFS_DIR}"

# Create directory structure
mkdir -p bin sbin etc proc sys dev run tmp root home var/log
mkdir -p usr/bin usr/sbin

# Copy binaries
cp "${REPO_ROOT}/target/release/ayux_init" ./init
cp "${REPO_ROOT}/target/release/login_manager" ./bin/
cp "${REPO_ROOT}/target/release/ayux_shell" ./bin/

# Create some basic files
echo "root:x:0:0:root:/root:/bin/ayux_shell" > etc/passwd
echo "ayux:x:1000:1000:ayux:/home/ayux:/bin/ayux_shell" >> etc/passwd

# Create a simple welcome message
echo "Welcome to AyuxOS Milestone 1" > etc/motd

# Pack initramfs
echo "Packing initramfs..."
find . | cpio -H newc -o | gzip > "${BUILD_DIR}/initramfs.cpio.gz"

echo "Initramfs generated at ${BUILD_DIR}/initramfs.cpio.gz"
