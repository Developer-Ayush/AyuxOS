#!/bin/bash
set -e

BUILD_DIR="$1"
REPO_ROOT="$2"

# Build target (defaults to production musl)
TARGET_ARCH="${TARGET:-x86_64-unknown-linux-musl}"
TARGET_DIR="${REPO_ROOT}/target/${TARGET_ARCH}/release"

ROOTFS_DIR="${BUILD_DIR}/rootfs"

echo "Assembling rootfs in ${ROOTFS_DIR}..."
echo "Using target: ${TARGET_ARCH}"

rm -rf "${ROOTFS_DIR}"
mkdir -p "${ROOTFS_DIR}"
cd "${ROOTFS_DIR}"

# Directory structure
mkdir -p \
    bin \
    sbin \
    etc \
    proc \
    sys \
    dev \
    run \
    tmp \
    root \
    home \
    var/log \
    main \
    users \
    usr/bin \
    usr/sbin

# Verify binaries exist
for BIN in \
    ayux_init \
    login_manager \
    ayux_shell \
    auth_service \
    session_manager \
    security_manager
do
    if [ ! -f "${TARGET_DIR}/${BIN}" ]; then
        echo "ERROR: Missing binary: ${TARGET_DIR}/${BIN}"
        exit 1
    fi
done

# Copy binaries
install -m 755 "${TARGET_DIR}/ayux_init" ./init
install -m 755 "${TARGET_DIR}/login_manager" ./bin/
install -m 755 "${TARGET_DIR}/ayux_shell" ./bin/
install -m 755 "${TARGET_DIR}/auth_service" ./bin/
install -m 755 "${TARGET_DIR}/session_manager" ./bin/
install -m 755 "${TARGET_DIR}/security_manager" ./bin/

# Basic system files
cat > etc/passwd <<EOF
root:x:0:0:root:/root:/bin/ayux_shell
EOF

cat > etc/motd <<EOF
Welcome to AyuxOS Milestone 2 - Security & Isolation
EOF

echo "Packing initramfs..."
find . -print0 \
    | cpio --null -o -H newc \
    | gzip -9 > "${BUILD_DIR}/initramfs.cpio.gz"

echo "Initramfs generated at ${BUILD_DIR}/initramfs.cpio.gz"