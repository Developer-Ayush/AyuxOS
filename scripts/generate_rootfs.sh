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
    ayux/apps \
    ayux/system \
    ayux/services \
    ayux/security \
    ayux/config \
    ayux/runtime \
    ayux/themes \
    ayux/cache \
    ayux/logs \
    ayux/updates \
    ayux/fonts \
    ayux/icons \
    ayux/certificates \
    ayux/libraries \
    ayux/manifests \
    ayux/native \
    ayux/media \
    ayux/devices \
    ayux/tmp \
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

# Core OS Services
install -m 755 "${TARGET_DIR}/log_service" ./ayux/services/
install -m 755 "${TARGET_DIR}/auth_service" ./ayux/services/
install -m 755 "${TARGET_DIR}/session_manager" ./ayux/services/
install -m 755 "${TARGET_DIR}/security_manager" ./ayux/services/
install -m 755 "${TARGET_DIR}/network_manager" ./ayux/services/
install -m 755 "${TARGET_DIR}/window_server" ./ayux/services/

# Native Applications
install -m 755 "${TARGET_DIR}/login_manager_gui" ./ayux/apps/
install -m 755 "${TARGET_DIR}/desktop" ./ayux/apps/
install -m 755 "${TARGET_DIR}/terminal_emulator" ./ayux/apps/
install -m 755 "${TARGET_DIR}/ayux_shell" ./ayux/apps/
# login_manager (CLI) - keeping it for now, but move it
install -m 755 "${TARGET_DIR}/login_manager" ./ayux/apps/

# Legacy/Compatibility symlinks (optional, but might help existing code before refactor)
ln -sf /ayux/services/log_service ./bin/log_service
ln -sf /ayux/services/auth_service ./bin/auth_service
ln -sf /ayux/services/session_manager ./bin/session_manager
ln -sf /ayux/services/security_manager ./bin/security_manager
ln -sf /ayux/services/network_manager ./bin/network_manager
ln -sf /ayux/services/window_server ./bin/window_server
ln -sf /ayux/apps/login_manager_gui ./bin/login_manager_gui
ln -sf /ayux/apps/desktop ./bin/desktop
ln -sf /ayux/apps/terminal_emulator ./bin/terminal_emulator
ln -sf /ayux/apps/ayux_shell ./bin/ayux_shell
ln -sf /ayux/apps/login_manager ./bin/login_manager

if [ -f "${REPO_ROOT}/ayux_assets/default.ttf" ]; then
    cp "${REPO_ROOT}/ayux_assets/default.ttf" ./ayux/assets/
    cp "${REPO_ROOT}/ayux_assets/default.ttf" ./ayux/fonts/
fi

cat > etc/passwd <<EOF
root:x:0:0:root:/root:/ayux/apps/ayux_shell
EOF

cat > etc/motd <<EOF
Welcome to AyuxOS Milestone 4 - Graphics Stack & UI Foundation
EOF

cat > ayux/config/services.toml <<EOF
[services.log_service]
path = "/ayux/services/log_service"
dependencies = []
restart_policy = "always"
priority = 1
health_check_socket = "/ayux/runtime/log.sock"

[services.auth_service]
path = "/ayux/services/auth_service"
dependencies = ["log_service"]
restart_policy = "always"
priority = 2
health_check_socket = "/ayux/runtime/auth.sock"

[services.session_manager]
path = "/ayux/services/session_manager"
dependencies = ["log_service"]
restart_policy = "always"
priority = 2
health_check_socket = "/ayux/runtime/session.sock"

[services.security_manager]
path = "/ayux/services/security_manager"
dependencies = ["session_manager", "log_service"]
restart_policy = "always"
priority = 3
health_check_socket = "/ayux/runtime/security.sock"

[services.network_manager]
path = "/ayux/services/network_manager"
dependencies = ["log_service"]
restart_policy = "always"
priority = 3
health_check_socket = "/ayux/runtime/network.sock"

[services.window_server]
path = "/ayux/services/window_server"
dependencies = ["log_service"]
restart_policy = "always"
priority = 4
health_check_socket = "/ayux/runtime/window_server.sock"
EOF

# Legacy/Compatibility link
ln -sf /ayux/config/services.toml etc/ayux_services.toml

echo "Packing initramfs..."
find . -print0 \
    | cpio --null -o -H newc \
    | gzip -9 > "${BUILD_DIR}/initramfs.cpio.gz"

echo "Initramfs generated at ${BUILD_DIR}/initramfs.cpio.gz"
