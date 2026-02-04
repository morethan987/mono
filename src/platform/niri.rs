//! Niri IPC integration for context awareness
//!
//! Provides methods to sense active window and workspace in niri compositor

use crate::error::{MonoError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

/// Client for niri's JSON-RPC IPC interface
pub struct NiriClient {
    socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u64,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub workspace_id: Option<u64>,
    pub is_focused: bool,
}

impl NiriClient {
    /// Create a new niri client, detecting socket from environment
    pub fn new() -> Result<Self> {
        let socket_path = env::var("NIRI_SOCKET").map_err(|_| {
            MonoError::Platform("NIRI_SOCKET not set. Is niri running?".to_string())
        })?;

        Ok(Self { socket_path })
    }

    /// Send a request to niri and get response
    fn request(&self, method: &str, params: Value) -> Result<Value> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| MonoError::Platform(format!("Failed to connect to niri socket: {}", e)))?;

        let request = json!({
            "method": method,
            "params": params
        });

        let req_str = serde_json::to_string(&request)?;
        stream.write_all(req_str.as_bytes())?;
        stream.write_all(b"\n")?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        let val: Value = serde_json::from_str(&response)?;

        if let Some(error) = val.get("error") {
            return Err(MonoError::Platform(format!("Niri IPC error: {}", error)));
        }

        Ok(val.get("data").cloned().unwrap_or(Value::Null))
    }

    /// Get all windows from niri
    pub fn get_windows(&self) -> Result<Vec<WindowInfo>> {
        let data = self.request("get-windows", json!({}))?;
        let windows: Vec<WindowInfo> = serde_json::from_value(data)?;
        Ok(windows)
    }

    /// Get the currently focused window
    pub fn get_focused_window(&self) -> Result<Option<WindowInfo>> {
        let windows = self.get_windows()?;
        Ok(windows.into_iter().find(|w| w.is_focused))
    }

    /// Get app_id of focused window
    pub fn get_active_app_id(&self) -> Result<Option<String>> {
        self.get_focused_window()
            .map(|opt| opt.and_then(|w| w.app_id))
    }

    /// Get title of focused window
    pub fn get_active_window_title(&self) -> Result<Option<String>> {
        self.get_focused_window()
            .map(|opt| opt.and_then(|w| w.title))
    }
}

/// Convenience function to sense current context app_id
pub fn get_current_app_id() -> Option<String> {
    NiriClient::new()
        .and_then(|client| client.get_active_app_id())
        .ok()
        .flatten()
}
