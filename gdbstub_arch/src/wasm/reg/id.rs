use core::num::NonZeroUsize;
use gdbstub::arch::RegId;

/// The only register exposed to GDB: `pc` (register index 0).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum WasmRegId {
    /// Program Counter.
    Pc,
}

impl RegId for WasmRegId {
    fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
        match id {
            0 => Some((WasmRegId::Pc, NonZeroUsize::new(8))),
            _ => None,
        }
    }

    fn to_raw_id(&self) -> Option<usize> {
        match self {
            WasmRegId::Pc => Some(0),
        }
    }
}
