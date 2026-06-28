/// Root of the immutable operating system.
pub const AYUX_ROOT: &str = "/ayux";

pub const AYUX_APPS: &str = "/ayux/apps";
pub const AYUX_SYSTEM: &str = "/ayux/system";
pub const AYUX_SERVICES: &str = "/ayux/services";
pub const AYUX_SECURITY: &str = "/ayux/security";
pub const AYUX_CONFIG: &str = "/ayux/config";
pub const AYUX_RUNTIME: &str = "/ayux/runtime";
pub const AYUX_THEMES: &str = "/ayux/themes";
pub const AYUX_CACHE: &str = "/ayux/cache";
pub const AYUX_LOGS: &str = "/ayux/logs";
pub const AYUX_UPDATES: &str = "/ayux/updates";
pub const AYUX_FONTS: &str = "/ayux/fonts";
pub const AYUX_ICONS: &str = "/ayux/icons";
pub const AYUX_CERTIFICATES: &str = "/ayux/certificates";
pub const AYUX_LIBRARIES: &str = "/ayux/libraries";
pub const AYUX_MANIFESTS: &str = "/ayux/manifests";
pub const AYUX_NATIVE: &str = "/ayux/native";
pub const AYUX_MEDIA: &str = "/ayux/media";
pub const AYUX_DEVICES: &str = "/ayux/devices";
pub const AYUX_TMP: &str = "/ayux/tmp";

/// Root for user data.
pub const USERS_ROOT: &str = "/users";

/// Root for administrator data.
pub const ROOT_ROOT: &str = "/root";

/// Service socket paths
pub const LOG_SOCKET: &str = "/ayux/runtime/log.sock";
pub const AUTH_SOCKET: &str = "/ayux/runtime/auth.sock";
pub const SESSION_SOCKET: &str = "/ayux/runtime/session.sock";
pub const SECURITY_SOCKET: &str = "/ayux/runtime/security.sock";
pub const NETWORK_SOCKET: &str = "/ayux/runtime/network.sock";
pub const WINDOW_SERVER_SOCKET: &str = "/ayux/runtime/window_server.sock";

/// Returns the home directory for a user given their internal ID.
pub fn user_home(internal_id: &str) -> String {
    format!("{}/{}", USERS_ROOT, internal_id)
}

/// Returns the AppData directory for a native application for a specific user.
pub fn user_app_data(internal_id: &str, app_name: &str) -> String {
    format!("{}/{}/AppData/{}", USERS_ROOT, internal_id, app_name)
}

/// Returns the Third-Party apps directory for a specific user.
pub fn user_apps(internal_id: &str) -> String {
    format!("{}/{}/Apps", USERS_ROOT, internal_id)
}

/// Returns the path to a system service executable.
pub fn service_executable(name: &str) -> String {
    format!("{}/{}", AYUX_SERVICES, name)
}

/// Returns the path to a native application executable.
pub fn app_executable(name: &str) -> String {
    format!("{}/{}", AYUX_APPS, name)
}
