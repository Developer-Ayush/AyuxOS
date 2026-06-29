# AyuxOS Permanent Filesystem Architecture

AyuxOS implements a permanent filesystem architecture designed for security, immutability, and clear separation of concerns.

## 1. Directory Structure

The root filesystem is divided into three primary regions:

### `/ayux/` - The Immutable Operating System
This directory contains all operating system resources. It is designed to be immutable and modified only via verified system updates.

*   `apps/`: Installed native system application executables (Shared by all users).
*   `services/`: Core operating system services (Auth, Session, Security, etc.).
*   `system/`: Low-level system binaries and tools.
*   `config/`: System-wide configuration files (e.g., `services.toml`).
*   `runtime/`: Transient runtime files such as IPC sockets and PIDs (mounted as `tmpfs`).
*   `security/`: System security resources (e.g., `system_secret`).
*   `libraries/`: Shared system libraries used by applications and services.
*   `native/`: Shared native resources, widgets, and platform assets.
*   `fonts/`: System-wide font assets.
*   `icons/`: System-wide icon assets.
*   `themes/`: OS theme definitions.
*   `cache/`: System-wide transient cache.
*   `logs/`: Persistent system logs.
*   `updates/`: Staging area for system updates.
*   `manifests/`: Application and system manifests.
*   `media/`: Built-in media assets (wallpapers, sounds).
*   `devices/`: Device-specific configurations and resources.
*   `tmp/`: System-level temporary directory (mounted as `tmpfs`).

### `/users/` - User Profiles & Data
Each user on the system has an isolated profile directory under `/users/<Internal-ID>/`.

*   `AppData/<AppName>/`: Isolated data storage for native applications.
*   `Apps/`: Isolated installation directory for Third-Party applications.
*   `Documents/`, `Pictures/`, `Music/`, etc.: Standard user data folders.

### `/root/` - Administrator Workspace
The private workspace for the system administrator. It is isolated from regular user data.

---

## 2. Application Architecture

### Native Applications
Native applications (e.g., Camera, Terminal, Settings) are part of the operating system.
*   **Executable**: Exactly one copy exists at `/ayux/apps/<app_name>`.
*   **Data**: User-specific data is stored in the user's profile at `/users/<Internal-ID>/AppData/<AppName>/`.
*   **Communication**: Native apps must only interact with the OS via Public APIs (HAL, AIPC, Services).

### Third-Party Applications
Third-Party applications are never shared between users.
*   **Installation**: Each user has an independent installation at `/users/<Internal-ID>/Apps/<AppName>/`.
*   **Isolation**: No executables, data, or settings are shared between different users' installations of the same third-party app.

---

## 3. Security Philosophy

1.  **Immutability**: The `/ayux` tree is immutable to users and applications.
2.  **Isolation**: Users cannot access each other's data or the administrator's workspace.
3.  **No Absolute Paths**: Applications and services should use the centralized path abstraction (`libayux::paths`) to interact with the filesystem, ensuring maintainability and flexibility.
