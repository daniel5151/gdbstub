use core::num::NonZeroUsize;

use gdbstub::arch::RegId;

/// RISC-V Register identifier.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum RiscvRegId<U> {
    /// General Purpose Register (x0-x31).
    Gpr(u8),
    /// Floating Point Register (f0-f31).
    Fpr(u8),
    /// Program Counter.
    Pc,
    /// Control and Status Register.
    Csr(u16),
    /// Privilege level.
    Priv,

    #[doc(hidden)]
    _Marker(core::marker::PhantomData<U>),
}

macro_rules! impl_riscv_reg_id {
    ($usize:ty) => {
        impl RegId for RiscvRegId<$usize> {
            fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
                const USIZE: usize = core::mem::size_of::<$usize>();

                let reg_size = match id {
                    0..=31 => (Self::Gpr(id as u8), Some(NonZeroUsize::new(USIZE).unwrap())),
                    32 => (Self::Pc, Some(NonZeroUsize::new(USIZE).unwrap())),
                    33..=64 => (
                        Self::Fpr((id - 33) as u8),
                        Some(NonZeroUsize::new(USIZE).unwrap()),
                    ),
                    65..=4160 => (
                        Self::Csr((id - 65) as u16),
                        Some(NonZeroUsize::new(USIZE).unwrap()),
                    ),
                    4161 => (Self::Priv, Some(NonZeroUsize::new(1).unwrap())),
                    _ => return None,
                };
                Some(reg_size)
            }
        }
    };
}

impl_riscv_reg_id!(u32);
impl_riscv_reg_id!(u64);
