# AyuxOS Architecture and Design Document

## 1. Introduction
AyuxOS is a security-first mobile operating system built on the Linux kernel. It focuses on maximum security, user freedom, and performance with a minimal footprint.

### Milestone 4: Graphics Stack & UI Foundation
The objective of this milestone is to build the graphics infrastructure that every future graphical component will rely on.
Key features:
- **Graphics HAL**: Enhanced to support Linux framebuffer and evdev input.
- **libgraphics**: A software rendering library for pixels, lines, shapes, and alpha blending.
- **Window Server & Compositor**: Manages window lifecycles and aggregates surfaces into the final display.
- **libui**: A widget-based UI toolkit with support for themes and layout.
- **Graphical Login Manager**: A replacement for the terminal login with a GUI.
- **Desktop Foundation**: The first AyuxOS desktop with wallpaper and taskbar.

## 2. High-Level Architecture
AyuxOS follows a layered architecture with strict boundary enforcement:

1.  **Hardware Layer**: Physical mobile hardware (ARM64 target, x86_64 for development).
2.  **Kernel Layer**: Linux 6.12 LTS with AyuxOS-specific security configurations and drivers.
3.  **Graphics HAL**: Unified interface for the graphics subsystem (Display, Input).
4.  **System Services Layer**: Core services (Init, Security Manager, IPC, Window Server, Compositor).
5.  **AyuxOS Framework / libui**: Native API for graphical application development.
6.  **Application Layer**: System apps (Login Manager, Desktop, Terminal) and User apps.

## 3. Graphics Pipeline
AyuxOS uses a modern, modular rendering pipeline:
1. **Application**: Uses `libui` to define the interface.
2. **libui**: Renders widgets into a **Shared Memory Surface**.
3. **Window Server**: Receives window events and manages window state via AIPC.
4. **Compositor**: Aggregates all window surfaces into a double-buffered framebuffer.
5. **Graphics HAL**: Provides access to the physical framebuffer device.

## 4. Security Model
The security model focuses on a tri-level hierarchy:
- **`/ayux`**: Immutable Operating System. Contains all OS binaries, services, and shared resources.
- **`/root`**: Isolated Administrator workspace.
- **`/users`**: User Data and Profiles. Each user is isolated in their own UUID-based directory.

For more details on the filesystem layout, see [FILESYSTEM.md](FILESYSTEM.md).

Window isolation is enforced by the Window Server, ensuring applications can only access their own surfaces.

## 5. Subsystems

### 5.1. Window Server
Dedicated service responsible for:
- Window lifecycle management (Create/Destroy).
- Input routing and focus management.
- Synchronization between clients and the compositor.

### 5.2. libgraphics
Software rendering engine implementing:
- Pixel manipulation and alpha blending.
- Geometry drawing (Lines, Rectangles, Circles).
- Clipping and Layers.
- Font rendering placeholder (intended for fontdue).
- Image loading placeholder (intended for image crate).

### 5.3. libui
Widget toolkit providing:
- Base Widget trait and event system.
- Standard widgets: Label, Button, Panel, etc.
- Theme engine for consistent look and feel.
