use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize};

/// PTY I/O handles for [`crate::view::TerminalView`].
pub struct PtyHandles {
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn Read + Send>,
    pub master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    pub child: Box<dyn Child + Send + Sync>,
}

pub fn spawn_terminal(
    shell: Option<String>,
    working_dir: Option<PathBuf>,
    rows: u16,
    cols: u16,
) -> Result<PtyHandles> {
    let pty_system = portable_pty::native_pty_system();

    let pty_size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(pty_size)
        .context("failed to open PTY")?;

    let shell_cmd = shell.unwrap_or_else(default_shell);

    let cwd = working_dir.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });

    let mut cmd = CommandBuilder::new(&shell_cmd);
    cmd.cwd(cwd);
    apply_shell_environment(&mut cmd, &shell_cmd);
    apply_interactive_shell_args(&mut cmd, &shell_cmd);

    let child = pair
        .slave
        .spawn_command(cmd)
        .context("failed to spawn shell in PTY")?;

    let writer = pair
        .master
        .take_writer()
        .context("failed to take PTY writer")?;
    let reader = pair
        .master
        .try_clone_reader()
        .context("failed to clone PTY reader")?;

    // Release the slave side so ConPTY keeps a single master reader/writer pair.
    drop(pair.slave);

    let master = Arc::new(Mutex::new(pair.master));

    Ok(PtyHandles {
        writer,
        reader,
        master,
        child,
    })
}

pub fn default_shell() -> String {
    if cfg!(windows) {
        windows_powershell_executable()
    } else if cfg!(target_os = "macos") {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}

/// Environment variables expected by interactive shells in a PTY.
fn apply_shell_environment(cmd: &mut CommandBuilder, shell_cmd: &str) {
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "RML");
    cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));

    if !cfg!(windows) {
        cmd.env(
            "LANG",
            std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string()),
        );
    }

    let _ = shell_cmd;
}

/// Launch shells in interactive mode so they emit banners, read profiles, and draw prompts.
fn apply_interactive_shell_args(cmd: &mut CommandBuilder, shell_cmd: &str) {
    let lower = shell_cmd.to_ascii_lowercase();
    let base = lower
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&lower);

    match base {
        // Do not pass -NoLogo / -NoProfile: those suppress the startup banner and profile output.
        "bash" | "bash.exe" => {
            cmd.arg("-i");
        }
        "zsh" | "zsh.exe" => {
            cmd.arg("-i");
        }
        _ => {}
    }
}

#[cfg(windows)]
fn windows_powershell_executable() -> String {
    if command_available("pwsh.exe") {
        "pwsh.exe".to_string()
    } else {
        "powershell.exe".to_string()
    }
}

#[cfg(not(windows))]
fn windows_powershell_executable() -> String {
    "powershell.exe".to_string()
}

#[cfg(windows)]
fn command_available(name: &str) -> bool {
    std::process::Command::new("where")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolve shell executable from a workbench resource schema.
pub fn shell_for_schema(schema: &str) -> String {
    match schema {
        "powershell" => windows_powershell_executable(),
        "cmd" => "cmd.exe".to_string(),
        "bash" => {
            if cfg!(windows) {
                std::env::var("SHELL").unwrap_or_else(|_| "bash.exe".to_string())
            } else {
                "/bin/bash".to_string()
            }
        }
        "zsh" => {
            if cfg!(windows) {
                std::env::var("SHELL").unwrap_or_else(|_| "zsh.exe".to_string())
            } else {
                "/bin/zsh".to_string()
            }
        }
        _ => default_shell(),
    }
}
