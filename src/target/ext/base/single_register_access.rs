use crate::arch::Arch;
use crate::target::{Target, TargetResult};

/// Target Extension - Support for single-register access.
///
/// While this is an optional feature, it is **highly recommended** to
/// implement it when possible, as it can significantly improve performance
/// on certain architectures.
///
/// If this extension is not implemented, the GDB client will fall-back to
/// accessing _all_ registers, even in cases where it only requires knowing a
/// single register's value.
///
/// Moreover, certain architectures have registers that are not accessible as
/// part of the default default register file used by the `read/write_registers`
/// methods, and can only be accessed via this extension (e.g: the RISC-V
/// Control and Status registers).
pub trait SingleRegisterAccess<Id>: Target {
    /// Read to a single register on the target.
    ///
    /// The `tid` field identifies which thread the value should be read from.
    /// On single threaded targets, `tid` is set to `()` and can be ignored.
    ///
    /// Implementations should write the value of the register using target's
    /// native byte order in the buffer `dst`.
    ///
    /// If the requested register could not be accessed, an appropriate
    /// non-fatal error should be returned.
    ///
    /// _Note:_ This method includes a stubbed default implementation which
    /// simply returns `Ok(())`. This is due to the fact that several built-in
    /// `arch` implementations haven't been updated with proper `RegId`
    /// implementations.
    fn read_register(
        &mut self,
        tid: Id,
        reg_id: <Self::Arch as Arch>::RegId,
        output: SendRegisterOutput<'_>,
    ) -> TargetResult<(), Self>;

    /// Write from a single register on the target.
    ///
    /// The `tid` field identifies which thread the value should be written to.
    /// On single threaded targets, `tid` is set to `()` and can be ignored.
    ///
    /// The `val` buffer contains the new value of the register in the target's
    /// native byte order. It is guaranteed to be the exact length as the target
    /// register.
    ///
    /// If the requested register could not be accessed, an appropriate
    /// non-fatal error should be returned.
    ///
    /// _Note:_ This method includes a stubbed default implementation which
    /// simply returns `Ok(())`. This is due to the fact that several built-in
    /// `arch` implementations haven't been updated with proper `RegId`
    /// implementations.
    fn write_register(
        &mut self,
        tid: Id,
        reg_id: <Self::Arch as Arch>::RegId,
        val: &[u8],
    ) -> TargetResult<(), Self>;
}

/// See [`SingleRegisterAccess`]
pub type SingleRegisterAccessOps<'a, Id, T> =
    &'a mut dyn SingleRegisterAccess<Id, Arch = <T as Target>::Arch, Error = <T as Target>::Error>;

/// An interface to send register data to the GDB remote debugger.
pub struct SendRegisterOutput<'a> {
    inner: &'a mut dyn FnMut(&[u8]),
}

impl<'a> SendRegisterOutput<'a> {
    pub(crate) fn new(inner: &'a mut dyn FnMut(&[u8])) -> Self {
        Self { inner }
    }

    /// Write out raw register bytes to the GDB debugger.
    pub fn write(&mut self, data: &[u8]) {
        (self.inner)(data)
    }
}
