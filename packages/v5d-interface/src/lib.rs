//! High-level API for controlling VEX V5 devices.
//!
//! The core feature this crate provides is the [`DeviceInterface`],
//! which exposes the functionality required to control a VEX device.
//!
//! It also exposes the [`connection`] module, which contains utilities for either
//! delegating requests regarding devices to another process or accepting
//! those requests using your own implementation of [`DeviceInterface`].

pub use vex_v5_serial::commands::file::ProgramData;

pub mod connection;
