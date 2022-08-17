//! `Register` structs for the AArch64 ARM architecture.

/// `RegId` definitions for the ARM AArch64 Architecture.
pub mod id;

mod aarch64_core;

pub use aarch64_core::AArch64CoreRegs;
