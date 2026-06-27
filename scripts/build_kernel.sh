#!/bin/bash
set -e

KERNEL_VERSION=$1
BUILD_DIR=$2
DEFCONFIG_PATH=$3

KERNEL_TARBALL="linux-${KERNEL_VERSION}.tar.xz"
KERNEL_URL="https://cdn.kernel.org/pub/linux/kernel/v6.x/${KERNEL_TARBALL}"

mkdir -p "${BUILD_DIR}"
cd "${BUILD_DIR}"

if [ ! -d "linux-${KERNEL_VERSION}" ]; then
    if [ ! -f "${KERNEL_TARBALL}" ]; then
        echo "Downloading kernel ${KERNEL_VERSION}..."
        wget "${KERNEL_URL}"
    fi
    echo "Extracting kernel..."
    tar -xf "${KERNEL_TARBALL}"
fi

cd "linux-${KERNEL_VERSION}"

echo "Applying AyuxOS defconfig..."
cp "${DEFCONFIG_PATH}" .config

echo "Building kernel..."
make -j$(nproc) bzImage
