# AyuxOS

AyuxOS is a security-first mobile operating system built on the Linux kernel. It focuses on maximum security, user freedom, and performance with a minimal footprint.

## Status: Foundation Security & Filesystem Architecture

This milestone establishes the permanent identity, privacy, filesystem, and security architecture of AyuxOS.

### Key Features
* **Privacy-First Identity**: User IDs are never stored in plaintext and are hidden even from the administrator.
* **Internal UUIDs**: All system components use random internal identifiers for isolation and privacy.
* **Immutable OS Core**: The operating system resides in a read-only, protected `/ayux` partition.
* **Isolated User Storage**: User data is stored in directories named by internal UUIDs, inaccessible to the administrator.
* **Secure Deletion**: Account deletion requires two-step authorization (Administrator + User).
* **Robust IPC**: AIPC provides secure, capability-based communication between services.

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