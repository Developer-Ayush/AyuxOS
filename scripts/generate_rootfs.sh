#!/bin/bash
set -e

BUILD_DIR="$1"
REPO_ROOT="$2"

TARGET_ARCH="${TARGET:-x86_64-unknown-linux-musl}"
TARGET_DIR="${REPO_ROOT}/target/${TARGET_ARCH}/release"

ROOTFS_DIR="${BUILD_DIR}/rootfs"

echo "Assembling rootfs in ${ROOTFS_DIR}..."
echo "Using target: ${TARGET_ARCH}"

rm -rf "${ROOTFS_DIR}"
mkdir -p "${ROOTFS_DIR}"
cd "${ROOTFS_DIR}"

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
    usr/sbin \
    dev/input \
    dev/shm \
    ayux/assets

for BIN in \
    ayux_init \
    login_manager \
    ayux_shell \
    auth_service \
    session_manager \
    security_manager \
    log_service \
    network_manager \
    window_server \
    login_manager_gui \
    desktop \
    terminal_emulator
do
    if [ ! -f "${TARGET_DIR}/${BIN}" ]; then
        echo "ERROR: Missing binary: ${TARGET_DIR}/${BIN}"
        exit 1
    fi
done

install -m 755 "${TARGET_DIR}/ayux_init" ./init
install -m 755 "${TARGET_DIR}/login_manager" ./bin/
install -m 755 "${TARGET_DIR}/ayux_shell" ./bin/
install -m 755 "${TARGET_DIR}/auth_service" ./bin/
install -m 755 "${TARGET_DIR}/session_manager" ./bin/
install -m 755 "${TARGET_DIR}/security_manager" ./bin/
install -m 755 "${TARGET_DIR}/log_service" ./bin/
install -m 755 "${TARGET_DIR}/network_manager" ./bin/
install -m 755 "${TARGET_DIR}/window_server" ./bin/
install -m 755 "${TARGET_DIR}/login_manager_gui" ./bin/
install -m 755 "${TARGET_DIR}/desktop" ./bin/
install -m 755 "${TARGET_DIR}/terminal_emulator" ./bin/

if [ -f "${REPO_ROOT}/ayux_assets/default.ttf" ]; then
    cp "${REPO_ROOT}/ayux_assets/default.ttf" ./ayux/assets/
fi

cat > etc/passwd <<EOF
root:x:0:0:root:/root:/bin/ayux_shell
EOF

cat > etc/motd <<EOF
Welcome to AyuxOS Milestone 4 - Graphics Stack & UI Foundation
EOF

cat > etc/ayux_services.toml <<EOF
[services.log_service]
path = "/bin/log_service"
dependencies = []
restart_policy = "always"
priority = 1
health_check_socket = "/run/log.sock"

[services.auth_service]
path = "/bin/auth_service"
dependencies = ["log_service"]
restart_policy = "always"
priority = 2
health_check_socket = "/run/auth.sock"

[services.session_manager]
path = "/bin/session_manager"
dependencies = ["log_service"]
restart_policy = "always"
priority = 2
health_check_socket = "/run/session.sock"

[services.security_manager]
path = "/bin/security_manager"
dependencies = ["session_manager", "log_service"]
restart_policy = "always"
priority = 3
health_check_socket = "/run/security.sock"

[services.network_manager]
path = "/bin/network_manager"
dependencies = ["log_service"]
restart_policy = "always"
priority = 3
health_check_socket = "/run/network.sock"

[services.window_server]
path = "/bin/window_server"
dependencies = ["log_service"]
restart_policy = "always"
priority = 4
health_check_socket = "/run/window_server.sock"
EOF

echo "Packing initramfs..."
find . -print0 \
    | cpio --null -o -H newc \
    | gzip -9 > "${BUILD_DIR}/initramfs.cpio.gz"

echo "Initramfs generated at ${BUILD_DIR}/initramfs.cpio.gz"
