#!/bin/bash
set -e

BUILD_DIR=$1
REPO_ROOT=$2
TARGET_ARCH="x86_64-unknown-linux-musl"

ROOTFS_DIR="${BUILD_DIR}/rootfs"

echo "Assembling rootfs in ${ROOTFS_DIR}..."

rm -rf "${ROOTFS_DIR}"
mkdir -p "${ROOTFS_DIR}"
cd "${ROOTFS_DIR}"

# Create directory structure
mkdir -p bin sbin etc proc sys dev run tmp root home var/log main users
mkdir -p usr/bin usr/sbin

# Copy binaries
cp "${REPO_ROOT}/target/${TARGET_ARCH}/release/ayux_init" ./init
cp "${REPO_ROOT}/target/${TARGET_ARCH}/release/login_manager" ./bin/
cp "${REPO_ROOT}/target/${TARGET_ARCH}/release/ayux_shell" ./bin/
cp "${REPO_ROOT}/target/${TARGET_ARCH}/release/auth_service" ./bin/
cp "${REPO_ROOT}/target/${TARGET_ARCH}/release/session_manager" ./bin/
cp "${REPO_ROOT}/target/${TARGET_ARCH}/release/security_manager" ./bin/

# Ensure init is executable
chmod +x ./init

# Create some basic files
# Milestone 2 uses its own auth database, but we keep etc/passwd for compatibility if needed
echo "root:x:0:0:root:/root:/bin/ayux_shell" > etc/passwd

# Create a simple welcome message
echo "Welcome to AyuxOS Milestone 2 - Security & Isolation" > etc/motd

# Pack initramfs
echo "Packing initramfs..."
find . | cpio -H newc -o | gzip > "${BUILD_DIR}/initramfs.cpio.gz"

echo "Initramfs generated at ${BUILD_DIR}/initramfs.cpio.gz"
