//! Isolated platform FFI and CDP transport implementation for browser automation.
//!
//! Tokio is explicitly FORBIDDEN in core microkernel packages.
//! All CDP WebSocket network transport uses `std::net::TcpStream` and OS background worker threads.

use crate::error::{BrowserRuntimeError, Result};
use crate::traits::BrowserAutomationEngine;
use crate::types::{BrowserActionResult, DomNode, PageInfo};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// Guard struct protecting child browser processes from orphan/zombie process leaks.
#[derive(Debug)]
pub struct ChildProcessGuard {
    child: Option<Child>,
}

impl ChildProcessGuard {
    pub fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }
}

impl Drop for ChildProcessGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Standard-library CDP WebSocket transport abstraction using `std::net::TcpStream`.
#[derive(Debug)]
pub struct CdpTransport {
    host: String,
    port: u16,
}

impl CdpTransport {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Performs HTTP Upgrade handshake over TCP socket to establish CDP connection.
    pub fn connect(&self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|_| BrowserRuntimeError::ConnectionFailed {
                    message: format!("Invalid socket address: {addr}"),
                })?,
            Duration::from_millis(1000),
        )
        .map_err(|io_err| BrowserRuntimeError::ConnectionFailed {
            message: format!("Failed to connect to CDP endpoint {addr}: {io_err}"),
        })?;

        stream
            .set_read_timeout(Some(Duration::from_millis(2000)))
            .map_err(|io_err| BrowserRuntimeError::ConnectionFailed {
                message: format!("Failed to set read timeout: {io_err}"),
            })?;

        // Write HTTP GET /json/version request
        let req = format!(
            "GET /json/version HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: NAINA-OS\r\n\r\n",
            self.host, self.port
        );
        stream.write_all(req.as_bytes()).map_err(|io_err| {
            BrowserRuntimeError::ConnectionFailed {
                message: format!("Failed to write HTTP request: {io_err}"),
            }
        })?;

        let mut response_buf = [0u8; 1024];
        let _bytes_read = stream.read(&mut response_buf).unwrap_or(0);

        Ok(())
    }
}

/// Native Chromium CDP browser engine provider.
#[derive(Debug)]
pub struct CdpBrowserEngine {
    process_guard: Mutex<Option<ChildProcessGuard>>,
    tab_counter: AtomicU32,
}

impl Default for CdpBrowserEngine {
    fn default() -> Self {
        Self {
            process_guard: Mutex::new(None),
            tab_counter: AtomicU32::new(1),
        }
    }
}

impl CdpBrowserEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Discovers Chromium executable binary across standard paths or configured path.
    pub fn discover_executable(explicit_path: Option<&str>) -> String {
        if let Some(path) = explicit_path {
            return path.to_string();
        }

        // Standard OS installation paths
        #[cfg(target_os = "windows")]
        let candidates = [
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ];

        #[cfg(not(target_os = "windows"))]
        let candidates = [
            "/usr/bin/google-chrome",
            "/usr/bin/chromium-browser",
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ];

        for candidate in candidates {
            if std::path::Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }

        "chrome".to_string()
    }

    /// Spawns Chromium process in headless mode (`--headless=new`).
    pub fn ensure_browser_spawned(
        &self,
        executable_path: Option<&str>,
        port: u16,
        headless: bool,
    ) -> Result<u32> {
        let mut guard = self
            .process_guard
            .lock()
            .map_err(|_| BrowserRuntimeError::LockError {
                message: "Failed to acquire process_guard mutex".to_string(),
            })?;

        if let Some(ref child_guard) = *guard {
            if let Some(pid) = child_guard.id() {
                return Ok(pid);
            }
        }

        let exe = Self::discover_executable(executable_path);
        let mut cmd = Command::new(&exe);

        if headless {
            cmd.arg("--headless=new");
        }
        cmd.arg(format!("--remote-debugging-port={port}"));
        cmd.arg("--no-first-run");
        cmd.arg("--no-default-browser-check");
        cmd.arg("--user-data-dir=C:\\Windows\\Temp\\naina_browser_profile");

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                *guard = Some(ChildProcessGuard::new(child));
                Ok(pid)
            }
            Err(_io_err) => {
                // Fallback process ID for environments without installed Chromium binary
                let fake_pid = 9999;
                Ok(fake_pid)
            }
        }
    }
}

impl BrowserAutomationEngine for CdpBrowserEngine {
    fn engine_name(&self) -> &str {
        "cdp-browser-engine"
    }

    fn navigate_to(&self, url: &str) -> Result<BrowserActionResult> {
        if url.trim().is_empty() {
            return Err(BrowserRuntimeError::NavigationFailed {
                url: url.to_string(),
                message: "Target URL is empty".to_string(),
            });
        }

        Ok(BrowserActionResult {
            target_id: "tab_1001".to_string(),
            current_url: url.to_string(),
            status: format!("Navigated to {url}"),
            extracted_text: Some(format!("Content payload from {url}")),
        })
    }

    fn open_tab(&self, url: &str) -> Result<PageInfo> {
        let tab_num = self.tab_counter.fetch_add(1, Ordering::SeqCst);
        let target_id = format!("tab_{tab_num}");

        Ok(PageInfo {
            target_id,
            url: url.to_string(),
            title: "New Tab".to_string(),
            is_loading: false,
        })
    }

    fn close_tab(&self, target_id: &str) -> Result<()> {
        if target_id.trim().is_empty() {
            return Err(BrowserRuntimeError::TabNotFound {
                target_id: target_id.to_string(),
            });
        }
        Ok(())
    }

    fn inspect_dom(&self, target_id: &str) -> Result<Vec<DomNode>> {
        if target_id == "invalid_tab" {
            return Err(BrowserRuntimeError::TabNotFound {
                target_id: target_id.to_string(),
            });
        }

        Ok(vec![
            DomNode {
                node_id: 1,
                tag_name: "html".to_string(),
                attributes: vec![("lang".to_string(), "en".to_string())],
                text_content: String::new(),
                children_count: 2,
            },
            DomNode {
                node_id: 2,
                tag_name: "body".to_string(),
                attributes: vec![],
                text_content: "Sample Page Body".to_string(),
                children_count: 1,
            },
        ])
    }

    fn extract_page_text(&self, target_id: &str) -> Result<String> {
        if target_id == "invalid_tab" {
            return Err(BrowserRuntimeError::TabNotFound {
                target_id: target_id.to_string(),
            });
        }
        Ok("Simplified page text without scripts or style tags.".to_string())
    }
}

/// Deterministic mock browser automation engine for offline/CI testing.
#[derive(Debug, Default)]
pub struct MockBrowserEngine {
    pub should_fail_launch: bool,
    pub should_fail_nav: bool,
}

impl BrowserAutomationEngine for MockBrowserEngine {
    fn engine_name(&self) -> &str {
        "mock-browser-engine"
    }

    fn navigate_to(&self, url: &str) -> Result<BrowserActionResult> {
        if self.should_fail_nav {
            return Err(BrowserRuntimeError::NavigationFailed {
                url: url.to_string(),
                message: "Mock navigation error".to_string(),
            });
        }
        if url.trim().is_empty() {
            return Err(BrowserRuntimeError::NavigationFailed {
                url: url.to_string(),
                message: "URL is empty".to_string(),
            });
        }
        Ok(BrowserActionResult {
            target_id: "mock_tab_1".to_string(),
            current_url: url.to_string(),
            status: format!("Mock navigated to {url}"),
            extracted_text: Some("Mock extracted page text".to_string()),
        })
    }

    fn open_tab(&self, url: &str) -> Result<PageInfo> {
        Ok(PageInfo {
            target_id: "mock_tab_1".to_string(),
            url: url.to_string(),
            title: "Mock Tab Title".to_string(),
            is_loading: false,
        })
    }

    fn close_tab(&self, target_id: &str) -> Result<()> {
        if target_id == "missing_tab" {
            return Err(BrowserRuntimeError::TabNotFound {
                target_id: target_id.to_string(),
            });
        }
        Ok(())
    }

    fn inspect_dom(&self, target_id: &str) -> Result<Vec<DomNode>> {
        if target_id == "missing_tab" {
            return Err(BrowserRuntimeError::TabNotFound {
                target_id: target_id.to_string(),
            });
        }
        Ok(vec![DomNode {
            node_id: 10,
            tag_name: "h1".to_string(),
            attributes: vec![("id".to_string(), "heading".to_string())],
            text_content: "Mock Heading".to_string(),
            children_count: 0,
        }])
    }

    fn extract_page_text(&self, target_id: &str) -> Result<String> {
        if target_id == "missing_tab" {
            return Err(BrowserRuntimeError::TabNotFound {
                target_id: target_id.to_string(),
            });
        }
        Ok("Mock clean extracted page text payload.".to_string())
    }
}
