# AyuxOS Milestone 1: Boot Foundation

This milestone establishes the core boot process and base system structure for AyuxOS.

## Features

- **Build System**: Top-level `Makefile` orchestrating kernel and userspace builds.
- **Kernel**: Linux 6.12 LTS with `ayux_defconfig`.
- **Ayux Init (PID 1)**: Custom Rust-based init system that mounts essential filesystems and manages system services.
- **Login Manager**: TTY-based authentication checking against `/etc/passwd`.
- **Ayux Shell**: Minimal TTY shell with built-in commands: `help`, `ls`, `cd`, `pwd`, `mkdir`, `touch`, `cat`, `echo`, `whoami`, `reboot`, `shutdown`.
- **AIPC**: Foundation for Ayux Inter-Process Communication using Unix Domain Sockets.
- **QEMU Integration**: Scripts to generate a bootable initramfs and run it in QEMU.

## Prerequisites

- `make`
- `cargo` (Rust)
- `wget`
- `tar`
- `cpio`
- `qemu-system-x86_64` (for running)
- `gcc`, `flex`, `bison`, `libelf-dev`, `libssl-dev` (for kernel compilation)

## Building

To build everything (Kernel + Userspace + Initramfs):

```bash
make
```

Note: The first build will take a significant amount of time as it downloads and compiles the Linux kernel.

## Running

To run the generated image in QEMU:

```bash
make run
```

Or manually:
```bash
./scripts/run_qemu.sh build/linux-6.12.11/arch/x86_64/boot/bzImage build/initramfs.cpio.gz
```

## Testing

Run unit tests:
```bash
cargo test --workspace
```

Run integration verification:
```bash
python3 tests/integration_test.py
```
