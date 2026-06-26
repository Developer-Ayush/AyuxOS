# AyuxOS Architecture and Design Document

## 1. Introduction
AyuxOS is a security-first mobile operating system built on the Linux kernel. It focuses on maximum security, user freedom, and performance with a minimal footprint.

## 2. High-Level Architecture
AyuxOS follows a layered architecture with strict boundary enforcement:

1.  **Hardware Layer**: Physical mobile hardware (ARM64 target, x86_64 for development).
2.  **Kernel Layer**: Linux 6.12 LTS with AyuxOS-specific security configurations and drivers.
3.  **Hardware Abstraction Layer (HAL)**: Unified interface for system services to interact with hardware.
4.  **System Services Layer**: Core services (Init, Security Manager, IPC, Display Compositor, Network Manager).
5.  **AyuxOS Framework**: Native API for application development.
6.  **Application Layer**: System apps and User sandboxed apps.

## 3. Security Model

### 3.1. Entity Hierarchy
-   **The System**: The trusted core. Owns the kernel, base OS, and system apps. Immutable at runtime.
-   **Root**: Restricted administrator. Manages users and updates. Cannot access user data or modify the system core.
-   **User**: Individual account with a fully isolated sandbox. Complete control within the sandbox, zero access outside.

### 3.2. Security Mechanisms
-   **Verified Boot**: Uses `dm-verity` to ensure integrity of the system partition. Digital signatures are verified at every stage.
-   **File-Based Encryption (FBE)**: Each user's data is encrypted using `fscrypt` with a key derived from their credentials.
-   **Sandboxing**: Uses Linux namespaces (PID, Mount, Network, UTS, IPC) and Cgroups to isolate user processes.
-   **Mandatory Access Control (MAC)**: AyuxOS custom policy enforced via kernel primitives to protect boundaries.

## 4. Subsystems

### 4.1. Ayux Init
Custom PID 1 responsible for:
-   Bootstrapping the system and mounting encrypted partitions.
-   Starting core system services in the correct order.
-   Managing service lifecycles and recovery.

### 4.2. Ayux IPC (AIPC)
A capability-based asynchronous message-passing system.
-   **Transport**: Unix Domain Sockets.
-   **Permissions**: Capability tokens passed via `SCM_RIGHTS`.
-   **Language**: Rust for safety and performance.

### 4.3. Ayux Display Compositor (ADC)
Custom mobile-optimized compositor.
-   **Protocol**: Minimalist Ayux-specific protocol (inspired by Wayland but simplified).
-   **Focus**: Low RAM usage and smooth 60fps animations.

### 4.4. Ayux HAL
-   Provides a stable C/Rust API.
-   Abstracts Linux-specific interfaces (sysfs, ioctl, etc.).
-   Ensures OS portability across different kernel versions and hardware.

## 5. Filesystem Layout (Runtime)
-   `/main/`: Immutable system components (Kernel, Base OS, Libraries).
-   `/root/`: Device administrator configuration and logs.
-   `/users/`: Encrypted user sandboxes.
    -   `/users/<username>/apps/`: User-installed applications.
    -   `/users/<username>/data/`: Private user data.
    -   `/users/<username>/logs/`: User-specific logs.

## 6. Compatibility Strategy
Modular translation layers:
1.  **Native**: Direct execution via AyuxOS Framework.
2.  **Linux**: Minimal syscall translation/mapping.
3.  **Android**: Runtime environment (ART-based) inside a container.
4.  **Windows/Apple**: Long-term milestones using binary translation and API mapping.

## 7. Build and Verification
-   **Build System**: `make` and `cargo`.
-   **Target**: Bootable QEMU image (raw or qcow2).
-   **Verification**: Automated testing suite for IPC, Security boundaries, and HAL consistency.
