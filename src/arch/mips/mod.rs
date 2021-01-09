//! Implementations for the MIPS architecture.

use crate::arch::Arch;
use crate::arch::RegId;

pub mod reg;

/// Implements `Arch` for 32-bit MIPS.
///
/// Check out the [module level docs](../index.html#whats-with-regidimpl) for
/// more info about the `RegIdImpl` type parameter.
pub enum Mips<RegIdImpl: RegId = reg::id::MipsRegId<u32>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

/// Implements `Arch` for 64-bit MIPS.
pub enum Mips64<RegIdImpl: RegId = reg::id::MipsRegId<u64>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

/// Implements `Arch` for 32-bit MIPS with the DSP feature enabled.
pub enum MipsWithDsp<RegIdImpl: RegId = reg::id::MipsRegId<u32>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

/// Implements `Arch` for 64-bit MIPS with the DSP feature enabled.
pub enum Mips64WithDsp<RegIdImpl: RegId = reg::id::MipsRegId<u64>> {
    #[doc(hidden)]
    _Marker(core::marker::PhantomData<RegIdImpl>),
}

impl<RegIdImpl: RegId> Arch for Mips<RegIdImpl> {
    type Usize = u32;
    type Registers = reg::MipsCoreRegs<u32>;
    type RegId = RegIdImpl;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>mips</architecture></target>"#)
    }
}

impl<RegIdImpl: RegId> Arch for Mips64<RegIdImpl> {
    type Usize = u64;
    type Registers = reg::MipsCoreRegs<u64>;
    type RegId = RegIdImpl;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>mips64</architecture></target>"#)
    }
}

impl<RegIdImpl: RegId> Arch for MipsWithDsp<RegIdImpl> {
    type Usize = u32;
    type Registers = reg::MipsCoreRegsWithDsp<u32>;
    type RegId = RegIdImpl;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>mips</architecture><feature name="org.gnu.gdb.mips.dsp"></feature></target>"#,
        )
    }
}

impl<RegIdImpl: RegId> Arch for Mips64WithDsp<RegIdImpl> {
    type Usize = u64;
    type Registers = reg::MipsCoreRegsWithDsp<u64>;
    type RegId = RegIdImpl;

    fn target_description_xml() -> Option<&'static str> {
        Some(
            r#"<target version="1.0"><architecture>mips64</architecture><feature name="org.gnu.gdb.mips.dsp"></feature></target>"#,
        )
    }
}
