#!/bin/bash
set -e

INITRAMFS_PATH=$1
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

echo "Verifying initramfs: ${INITRAMFS_PATH}"

# Unpack initramfs
gunzip -c "${INITRAMFS_PATH}" | (cd "${TEMP_DIR}" && cpio -id)

# 1. Verify /init exists and is executable
if [ ! -x "${TEMP_DIR}/init" ]; then
    echo "ERROR: /init is missing or not executable in initramfs"
    exit 1
fi

# 2. Verify /init is a static ELF binary
if ! file "${TEMP_DIR}/init" | grep -E -q "statically linked|static-pie linked"; then
    echo "ERROR: /init is not statically linked"
    file "${TEMP_DIR}/init"
    exit 1
fi

# 3. Verify required directories exist
REQUIRED_DIRS=("main" "root" "users" "proc" "sys" "dev" "tmp" "run" "bin" "etc")
for dir in "${REQUIRED_DIRS[@]}"; do
    if [ ! -d "${TEMP_DIR}/${dir}" ]; then
        echo "ERROR: Required directory /${dir} is missing in initramfs"
        exit 1
    fi
done

# 4. Verify required binaries exist
REQUIRED_BINS=("bin/login_manager" "bin/ayux_shell" "bin/auth_service" "bin/session_manager" "bin/security_manager")
for bin in "${REQUIRED_BINS[@]}"; do
    if [ ! -f "${TEMP_DIR}/${bin}" ]; then
        echo "ERROR: Required binary /${bin} is missing in initramfs"
        exit 1
    fi
done

echo "Initramfs verification PASSED"
