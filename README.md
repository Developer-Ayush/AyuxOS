# AyuxOS

AyuxOS is a security-first mobile operating system built on the Linux kernel. It focuses on maximum security, user freedom, and performance with a minimal footprint.

## Status: Snow Leopard Release (Final Polish)

The Snow Leopard release provides a stable, professional, and production-quality command-line foundation.

### Key Features
* **Robust IPC**: AIPC (Ayux Inter-Process Communication) provides a versioned, enveloped protocol for secure service interaction.
* **Centralized Security**: The Security Manager authorizes all system operations, including filesystem access and power management.
* **Professional CLI**: A consistent terminal style with standardized prompts and headers across all system components.
* **Integrated Logging**: A transparent Log Service with rotation and module-specific logs.
* **Safety First**: Written primarily in Rust with a focus on robust error handling and zero compiler warnings.

## Getting Started

### Prerequisites
* Rust (latest stable)
* GCC
* GNU Make
* QEMU (for testing)

### Building
```bash
make
```

### Running
```bash
make run
```

## Documentation
* [Architecture](ARCHITECTURE.md)
* [Repository Structure](REPO_STRUCTURE.md)
* [Roadmap](ROADMAP.md)