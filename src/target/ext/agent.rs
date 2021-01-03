//! TODO: More docs

use crate::arch::Arch;
#[allow(unused_imports)] // used for intra-doc linking.
use crate::target::TargetError;
use crate::target::{Target, TargetResult};

/// A unique id identifying a bytecode expression registered with the agent.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash)]
pub struct BytecodeId(pub core::num::NonZeroUsize);

impl BytecodeId {
    /// Convenience method to avoid having to write something like
    /// `Some(BytecodeId(core::num::NonZeroUsize::new(id)?))`.
    pub fn new(id: usize) -> Option<BytecodeId> {
        Some(BytecodeId(core::num::NonZeroUsize::new(id)?))
    }
}

/// TODO: More docs
pub trait Agent: Target {
    /// Turn on or off the agent as a helper to perform some debugging
    /// operations delegated from GDB.
    fn enabled(&mut self, enabled: bool) -> Result<(), Self::Error>;

    /// Register a bytecode expression with the agent.
    ///
    /// Implementors can choose whether or not to validate bytecode expression
    /// as part of registration. If validation is performed as part of
    /// registration and the bytecode expression is found to be malformed,
    /// an appropriate non-fatal [`TargetError`] should be returned.
    ///
    /// On resource constrained targets where there might not be enough space to
    /// register the expression, an appropriate non-fatal [`TargetError`] should
    /// be returned (e.g: `TargetError::Errno(28)` - corresponding to `ENOSPC`).
    fn register_bytecode(&mut self, bytecode: &[u8]) -> TargetResult<BytecodeId, Self>;

    /// Remove a registered bytecode expression.
    ///
    /// If the specified `id` does not correspond to previously registered
    /// bytecode expression, a non-fatal [`TargetError`] should be returned.
    fn unregister_bytecode(&mut self, id: BytecodeId) -> TargetResult<(), Self>;

    /// Evaluate the specified bytecode expression.
    ///
    /// If the bytecode expression could not be executed, it is implementation
    /// defined whether or not a fatal or non-fatal error should be returned.
    fn evaluate(&mut self, id: BytecodeId) -> TargetResult<<Self::Arch as Arch>::Usize, Self>;
}

define_ext!(AgentOps, Agent);
