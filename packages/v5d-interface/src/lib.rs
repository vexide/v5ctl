//! High-level API for controlling VEX V5 devices.
//!
//! The core feature this crate provides is the [`DeviceInterface`],
//! which exposes the functionality required to control a VEX device.
//!
//! It also exposes the [`connection`] module, which contains utilities for either
//! delegating requests regarding devices to another process or accepting
//! those requests using your own implementation of [`DeviceInterface`].

use std::future::Future;

use serde::{Deserialize, Serialize};
use vex_v5_serial::packets::file::FileExitAction;

pub use vex_v5_serial::commands::file::ProgramData;

pub use crate::error::Result;

pub mod connection;
pub mod error;

/// A trait for communicating with VEX devices.
///
/// Structs that send commands to a VEX device may implement this trait.
/// This can either be done directly (i.e. over serial) or indirectly (for example,
/// by connecting to a remote daemon).
///
/// This crate provides [`DaemonConnection`](connection::DaemonConnection), an
/// implementation of this trait that delegates these actions to another process.
pub trait DeviceInterface {
    fn mock_tap(&mut self, x: u16, y: u16) -> impl Future<Output = Result> + Send;
    fn upload_program(
        &mut self,
        opts: UploadProgramOpts,
        handle_progress: impl FnMut(TransferProgress) + Send,
    ) -> impl Future<Output = Result> + Send;
    fn shutdown(&mut self) -> impl Future<Output = Result> + Send;
    fn request_pair(&mut self) -> impl Future<Output = Result> + Send;
    fn pairing_pin(&mut self, pin: [u8; 4]) -> impl Future<Output = Result> + Send;
    fn reconnect(&mut self) -> impl Future<Output = Result> + Send;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadProgramOpts {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub program_type: String,
    /// 1-indexed slot
    pub slot: u8,
    pub compression: bool,
    pub after_upload: AfterFileUpload,
    pub data: ProgramData,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AfterFileUpload {
    DoNothing,
    RunProgram,
    ShowRunScreen,
    Halt,
}
impl From<AfterFileUpload> for FileExitAction {
    fn from(value: AfterFileUpload) -> Self {
        match value {
            AfterFileUpload::DoNothing => FileExitAction::DoNothing,
            AfterFileUpload::RunProgram => FileExitAction::RunProgram,
            AfterFileUpload::ShowRunScreen => FileExitAction::ShowRunScreen,
            AfterFileUpload::Halt => FileExitAction::Halt,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum UploadStep {
    Ini,
    Bin,
    Lib,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransferProgress {
    pub percent: f32,
    pub step: UploadStep,
}
