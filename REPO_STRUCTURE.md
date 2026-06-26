# AyuxOS Source Repository Structure

```text
/
├── ARCHITECTURE.md          # High-level design document
├── ROADMAP.md               # Development milestones
├── Makefile                 # Top-level build script
├── build/                   # Build artifacts and intermediate files
├── scripts/                 # Utility scripts for image generation and QEMU
│   ├── generate_rootfs.sh
│   └── run_qemu.sh
├── kernel/                  # Linux kernel source and configuration
│   └── ayux_defconfig
├── base_os/                 # Core system files and configurations
├── init/                    # Ayux Init source code (Rust/C++)
├── hal/                     # Hardware Abstraction Layer source
├── libraries/               # Shared system libraries (AIPC, etc.)
│   ├── libaipc/
│   └── libayux/
├── system_apps/             # Source for built-in system apps
│   ├── login_manager/
│   └── ayux_shell/
├── sdk/                     # AyuxOS Native SDK
└── tests/                   # Integration and unit tests
```
