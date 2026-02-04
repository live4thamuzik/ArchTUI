//! command_executor.rs - Manages the execution of blocking shell commands in a dedicated background thread.
//!
//! This module provides the necessary types and a function to spawn a thread that listens for
//! `CommandRequest`s, executes the requested shell command (which is blocking), and sends
//! the `CommandResponse` back to the main application thread. This ensures the main TUI
//! thread remains responsive and prevents freezing during I/O operations.

use crate::process_guard::CommandProcessGroup;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use log::{debug, error};

/// Unique identifier for each command request/response pair.
pub type CommandId = u64;

/// Represents the type of shell command to be executed.
#[derive(Debug, Clone)]
pub enum CommandType {
    /// Corresponds to `lsblk -d -n -o NAME,SIZE,TYPE,RO,TRAN`
    LsblkAll,
    /// Corresponds to `lsblk -d -n -o MODEL <disk>`
    LsblkModel(String),
    /// Corresponds to `lsblk -d -n -o SIZE <disk>`
    LsblkSize(String),
    /// Corresponds to `lsblk -n -o NAME,TYPE,SIZE <disk>` (for partitions)
    LsblkPartitions(String),
    /// Corresponds to `fdisk -l <disk>`
    FdiskL(String),
    /// Corresponds to `mount`
    Mount,
    /// Corresponds to `blockdev --getss <disk>`
    BlockdevSs(String),
    // Add other command types as needed
}

/// A request to execute a blocking shell command.
#[derive(Debug)]
pub struct CommandRequest {
    pub id: CommandId,
    pub command_type: CommandType,
}

/// The response containing the result of a shell command execution.
#[derive(Debug)]
pub struct CommandResponse {
    pub id: CommandId,
    pub result: Result<String, String>, // Ok(stdout_output), Err(error_message)
}

/// Spawns a dedicated thread to execute blocking shell commands.
///
/// This thread continuously listens for `CommandRequest`s on `request_rx`.
/// Upon receiving a request, it executes the specified command and sends
/// a `CommandResponse` back via `response_tx`.
pub fn spawn_executor_thread(
    request_rx: Receiver<CommandRequest>,
    response_tx: Sender<CommandResponse>,
) {
    std::thread::spawn(move || {
        debug!("Command executor thread started.");
        while let Ok(request) = request_rx.recv() {
            debug!("Executing command request: {:?}", request);
            let response_result = match request.command_type {
                CommandType::LsblkAll => {
                    Command::new("lsblk")
                        .args(["-d", "-n", "-o", "NAME,SIZE,TYPE,RO,TRAN"])
                        .in_new_process_group()
                        .output()
                        .map_err(|e| e.to_string())
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).to_string())
                            }
                        })
                }
                CommandType::LsblkModel(disk) => {
                    Command::new("lsblk")
                        .args(["-d", "-n", "-o", "MODEL", &disk])
                        .in_new_process_group()
                        .output()
                        .map_err(|e| e.to_string())
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).to_string())
                            }
                        })
                }
                CommandType::LsblkSize(disk) => {
                    Command::new("lsblk")
                        .args(["-d", "-n", "-o", "SIZE", &disk])
                        .in_new_process_group()
                        .output()
                        .map_err(|e| e.to_string())
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).to_string())
                            }
                        })
                }
                CommandType::LsblkPartitions(disk) => {
                    Command::new("lsblk")
                        .args(["-n", "-o", "NAME,TYPE,SIZE", &disk])
                        .in_new_process_group()
                        .output()
                        .map_err(|e| e.to_string())
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).to_string())
                            }
                        })
                }
                CommandType::FdiskL(disk) => {
                    Command::new("fdisk")
                        .args(["-l", &disk])
                        .in_new_process_group()
                        .output()
                        .map_err(|e| e.to_string())
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).to_string())
                            }
                        })
                }
                CommandType::Mount => {
                    Command::new("mount")
                        .in_new_process_group()
                        .output()
                        .map_err(|e| e.to_string())
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).to_string())
                            }
                        })
                }
                CommandType::BlockdevSs(disk) => {
                    Command::new("blockdev")
                        .args(["--getss", &disk])
                        .in_new_process_group()
                        .output()
                        .map_err(|e| e.to_string())
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).to_string())
                            }
                        })
                }
            };

            if response_tx.send(CommandResponse { id: request.id, result: response_result }).is_err() {
                error!("Failed to send command response. Receiver probably dropped.");
                break; // Exit thread if main app is gone
            }
        }
        debug!("Command executor thread shut down.");
    });
}