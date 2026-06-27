# AyuxOS Root Makefile

.PHONY: all clean kernel initramfs run

BUILD_DIR = $(CURDIR)/build
ROOTFS_DIR = $(BUILD_DIR)/rootfs
KERNEL_VERSION = 6.12.11
KERNEL_DIR = $(BUILD_DIR)/linux-$(KERNEL_VERSION)
KERNEL_IMAGE = $(KERNEL_DIR)/arch/x86_64/boot/bzImage

CARGO = cargo
TARGET = x86_64-unknown-linux-musl
CARGO_OPTS = --release --target $(TARGET)

all: kernel initramfs

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# Kernel targets
kernel: $(KERNEL_IMAGE)

$(KERNEL_IMAGE): $(BUILD_DIR)
	./scripts/build_kernel.sh $(KERNEL_VERSION) $(BUILD_DIR) $(CURDIR)/kernel/ayux_defconfig

# Userspace targets
initramfs: $(BUILD_DIR)
	$(CARGO) build $(CARGO_OPTS)
	./scripts/generate_rootfs.sh $(BUILD_DIR) $(CURDIR)
	./scripts/verify_initramfs.sh $(BUILD_DIR)/initramfs.cpio.gz

clean:
	rm -rf $(BUILD_DIR)
	$(CARGO) clean

run: all
	./scripts/run_qemu.sh $(KERNEL_IMAGE) $(BUILD_DIR)/initramfs.cpio.gz
