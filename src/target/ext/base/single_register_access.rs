//! Support for single-register read/write access.

use crate::arch::Arch;
use crate::target::Target;
use crate::target::TargetResult;

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
pub trait SingleRegisterAccess<Tid>: Target
where
    Tid: crate::is_valid_tid::IsValidTid,
{
    /// Read to a single register on the target.
    ///
    /// The `tid` field identifies which thread the value should be read from.
    /// On single threaded targets, `tid` is set to `()` and can be ignored.
    ///
    /// Implementations should write the value of the register using target's
    /// native byte order in the buffer `buf`.
    ///
    /// Return the number of bytes written into `buf` or `0` if the register is
    /// valid but unavailable.
    ///
    /// If the requested register could not be accessed, an appropriate
    /// non-fatal error should be returned.
    fn read_register(
        &mut self,
        tid: Tid,
        reg_id: <Self::Arch as Arch>::RegId,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self>;

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
    fn write_register(
        &mut self,
        tid: Tid,
        reg_id: <Self::Arch as Arch>::RegId,
        val: &[u8],
    ) -> TargetResult<(), Self>;
}

/// See [`SingleRegisterAccess`]
pub type SingleRegisterAccessOps<'a, Tid, T> =
    &'a mut dyn SingleRegisterAccess<Tid, Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
