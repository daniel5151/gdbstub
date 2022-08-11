use core::num::NonZeroUsize;

use gdbstub::arch::RegId;

/// AArch64 Architectural Registers.
///
/// Represents architectural registers as
///
/// - individual variants for those described in section B1.2. _Registers in
///   AArch64 Execution state_ of the Architecture Reference Manual (DDI
///   0487H.a), accessed through their own respective subsets of instructions
///   _e.g._ GPRs, FP & SIMD, ...
/// - a generic variant for system registers, accessed through MSR/MRS
///   instructions, based on their encoding as described in section C5.1. _The
///   System instruction class encoding space_ when `op0` is `0b10` (_Debug and
///   trace registers_) or `0b11` (_Non-debug System registers_ and
///   _Special-purpose registers_), as `0b0x` do not encode registers;
/// - a variant for the abstraction of process state information, `PSTATE`
///   (section D1.4.), which should be preferred over field-specific
///   special-purpose registers (`NZCV`, `DAIF`, ...)
///
/// Provides `const` aliases for most system registers as syntactic sugar for
/// the `System` variant. When those aren't available (_e.g._ for newly-added
/// registers), the literal representation `System(0baa_bbb_xxxx_yyyy_cc)` may
/// be used, similarly to the standard assembly symbol,
/// `S<op0>_<op1>_<CRn>_<CRm>_<op2>`.
///
/// To future-proof and greatly simplify the implementation, the target's XML
/// must encode system registers by using their 16-bit encoding as the `regnum`
/// property; no clash with architectural registers is possible as the top bit
/// of the 16-bit value is guaranteed to be set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AArch64RegId {
    /// General-purpose Register File (X0 - X30)
    X(u8),
    /// Stack Pointer
    Sp,
    /// Program Counter
    Pc,
    /// Process State (Pseudo-Register)
    Pstate,
    /// SIMD & FP Register File (V0 - V31)
    V(u8),
    /// System Registers encoded as (Op0:2, Op1:3, CRn:4, CRm:4, Op2:2)
    System(u16),
}

impl RegId for AArch64RegId {
    fn from_raw_id(id: usize) -> Option<(Self, Option<NonZeroUsize>)> {
        let reg = match id {
            0..=30 => Self::X(id as u8),
            31 => Self::Sp,
            32 => Self::Pc,
            33 => Self::Pstate,
            34..=65 => Self::V((id - 34) as u8),
            66 => Self::FPSR,
            67 => Self::FPCR,
            #[allow(clippy::unusual_byte_groupings)]
            // We configure GDB to use regnums that correspond to the architectural u16 opcode
            // and avoid clashes with core registers thanks to op0==0b00 and op0==0b01 not being
            // allocated for system registers.
            0b10_000_0000_0000_000..=0b11_111_1111_1111_111 => Self::System(id as u16),
            _ => return None,
        };

        Some((reg, Some(NonZeroUsize::new(reg.len()?)?)))
    }
}

#[allow(clippy::unusual_byte_groupings)]
impl AArch64RegId {
    #[allow(clippy::len_without_is_empty)]
    /// Gives the size of the register.
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::Pstate => Some(core::mem::size_of::<u32>()),
            Self::X(_n @ 0..=30) => Some(core::mem::size_of::<u64>()),
            Self::V(_n @ 0..=31) => Some(core::mem::size_of::<u128>()),
            Self::Pc | Self::Sp | Self::System(_) => Some(core::mem::size_of::<u64>()),
            _ => None,
        }
    }

    /// Main ID Register
    pub const MIDR_EL1: Self = Self::System(0b11_000_0000_0000_000);
    /// Multiprocessor Affinity Register
    pub const MPIDR_EL1: Self = Self::System(0b11_000_0000_0000_101);
    /// Revision ID Register
    pub const REVIDR_EL1: Self = Self::System(0b11_000_0000_0000_110);
    /// AArch32 Processor Feature Register 0
    pub const ID_PFR0_EL1: Self = Self::System(0b11_000_0000_0001_000);
    /// AArch32 Processor Feature Register 1
    pub const ID_PFR1_EL1: Self = Self::System(0b11_000_0000_0001_001);
    /// AArch32 Debug Feature Register 0
    pub const ID_DFR0_EL1: Self = Self::System(0b11_000_0000_0001_010);
    /// AArch32 Auxiliary Feature Register 0
    pub const ID_AFR0_EL1: Self = Self::System(0b11_000_0000_0001_011);
    /// AArch32 Memory Model Feature Register 0
    pub const ID_MMFR0_EL1: Self = Self::System(0b11_000_0000_0001_100);
    /// AArch32 Memory Model Feature Register 1
    pub const ID_MMFR1_EL1: Self = Self::System(0b11_000_0000_0001_101);
    /// AArch32 Memory Model Feature Register 2
    pub const ID_MMFR2_EL1: Self = Self::System(0b11_000_0000_0001_110);
    /// AArch32 Memory Model Feature Register 3
    pub const ID_MMFR3_EL1: Self = Self::System(0b11_000_0000_0001_111);
    /// AArch32 Instruction Set Attribute Register 0
    pub const ID_ISAR0_EL1: Self = Self::System(0b11_000_0000_0010_000);
    /// AArch32 Instruction Set Attribute Register 1
    pub const ID_ISAR1_EL1: Self = Self::System(0b11_000_0000_0010_001);
    /// AArch32 Instruction Set Attribute Register 2
    pub const ID_ISAR2_EL1: Self = Self::System(0b11_000_0000_0010_010);
    /// AArch32 Instruction Set Attribute Register 3
    pub const ID_ISAR3_EL1: Self = Self::System(0b11_000_0000_0010_011);
    /// AArch32 Instruction Set Attribute Register 4
    pub const ID_ISAR4_EL1: Self = Self::System(0b11_000_0000_0010_100);
    /// AArch32 Instruction Set Attribute Register 5
    pub const ID_ISAR5_EL1: Self = Self::System(0b11_000_0000_0010_101);
    /// AArch32 Memory Model Feature Register 4
    pub const ID_MMFR4_EL1: Self = Self::System(0b11_000_0000_0010_110);
    /// AArch32 Instruction Set Attribute Register 6
    pub const ID_ISAR6_EL1: Self = Self::System(0b11_000_0000_0010_111);
    /// AArch32 Media And VFP Feature Register 0
    pub const MVFR0_EL1: Self = Self::System(0b11_000_0000_0011_000);
    /// AArch32 Media And VFP Feature Register 1
    pub const MVFR1_EL1: Self = Self::System(0b11_000_0000_0011_001);
    /// AArch32 Media And VFP Feature Register 2
    pub const MVFR2_EL1: Self = Self::System(0b11_000_0000_0011_010);
    /// AArch32 Processor Feature Register 2
    pub const ID_PFR2_EL1: Self = Self::System(0b11_000_0000_0011_100);
    /// Debug Feature Register 1
    pub const ID_DFR1_EL1: Self = Self::System(0b11_000_0000_0011_101);
    /// AArch32 Memory Model Feature Register 5
    pub const ID_MMFR5_EL1: Self = Self::System(0b11_000_0000_0011_110);
    /// AArch64 Processor Feature Register 0
    pub const ID_AA64PFR0_EL1: Self = Self::System(0b11_000_0000_0100_000);
    /// AArch64 Processor Feature Register 1
    pub const ID_AA64PFR1_EL1: Self = Self::System(0b11_000_0000_0100_001);
    /// SVE Feature ID Register 0
    pub const ID_AA64ZFR0_EL1: Self = Self::System(0b11_000_0000_0100_100);
    /// SME Feature ID Register 0
    pub const ID_AA64SMFR0_EL1: Self = Self::System(0b11_000_0000_0100_101);
    /// AArch64 Debug Feature Register 0
    pub const ID_AA64DFR0_EL1: Self = Self::System(0b11_000_0000_0101_000);
    /// AArch64 Debug Feature Register 1
    pub const ID_AA64DFR1_EL1: Self = Self::System(0b11_000_0000_0101_001);
    /// AArch64 Auxiliary Feature Register 0
    pub const ID_AA64AFR0_EL1: Self = Self::System(0b11_000_0000_0101_100);
    /// AArch64 Auxiliary Feature Register 1
    pub const ID_AA64AFR1_EL1: Self = Self::System(0b11_000_0000_0101_101);
    /// AArch64 Instruction Set Attribute Register 0
    pub const ID_AA64ISAR0_EL1: Self = Self::System(0b11_000_0000_0110_000);
    /// AArch64 Instruction Set Attribute Register 1
    pub const ID_AA64ISAR1_EL1: Self = Self::System(0b11_000_0000_0110_001);
    /// AArch64 Instruction Set Attribute Register 2
    pub const ID_AA64ISAR2_EL1: Self = Self::System(0b11_000_0000_0110_010);
    /// AArch64 Memory Model Feature Register 0
    pub const ID_AA64MMFR0_EL1: Self = Self::System(0b11_000_0000_0111_000);
    /// AArch64 Memory Model Feature Register 1
    pub const ID_AA64MMFR1_EL1: Self = Self::System(0b11_000_0000_0111_001);
    /// AArch64 Memory Model Feature Register 2
    pub const ID_AA64MMFR2_EL1: Self = Self::System(0b11_000_0000_0111_010);
    /// System Control Register (EL1)
    pub const SCTLR_EL1: Self = Self::System(0b11_000_0001_0000_000);
    /// Auxiliary Control Register (EL1)
    pub const ACTLR_EL1: Self = Self::System(0b11_000_0001_0000_001);
    /// Architectural Feature Access Control Register
    pub const CPACR_EL1: Self = Self::System(0b11_000_0001_0000_010);
    /// Random Allocation Tag Seed Register
    pub const RGSR_EL1: Self = Self::System(0b11_000_0001_0000_101);
    /// Tag Control Register
    pub const GCR_EL1: Self = Self::System(0b11_000_0001_0000_110);
    /// SVE Control Register (EL1)
    pub const ZCR_EL1: Self = Self::System(0b11_000_0001_0010_000);
    /// Trace Filter Control Register (EL1)
    pub const TRFCR_EL1: Self = Self::System(0b11_000_0001_0010_001);
    /// Streaming Mode Priority Register
    pub const SMPRI_EL1: Self = Self::System(0b11_000_0001_0010_100);
    /// SME Control Register (EL1)
    pub const SMCR_EL1: Self = Self::System(0b11_000_0001_0010_110);
    /// Translation Table Base Register 0 (EL1)
    pub const TTBR0_EL1: Self = Self::System(0b11_000_0010_0000_000);
    /// Translation Table Base Register 1 (EL1)
    pub const TTBR1_EL1: Self = Self::System(0b11_000_0010_0000_001);
    /// Translation Control Register (EL1)
    pub const TCR_EL1: Self = Self::System(0b11_000_0010_0000_010);
    /// Pointer Authentication Key A For Instruction (bits[63:0])
    pub const APIAKEYLO_EL1: Self = Self::System(0b11_000_0010_0001_000);
    /// Pointer Authentication Key A For Instruction (bits[127:64])
    pub const APIAKEYHI_EL1: Self = Self::System(0b11_000_0010_0001_001);
    /// Pointer Authentication Key B For Instruction (bits[63:0])
    pub const APIBKEYLO_EL1: Self = Self::System(0b11_000_0010_0001_010);
    /// Pointer Authentication Key B For Instruction (bits[127:64])
    pub const APIBKEYHI_EL1: Self = Self::System(0b11_000_0010_0001_011);
    /// Pointer Authentication Key A For Data (bits[63:0])
    pub const APDAKEYLO_EL1: Self = Self::System(0b11_000_0010_0010_000);
    /// Pointer Authentication Key A For Data (bits[127:64])
    pub const APDAKEYHI_EL1: Self = Self::System(0b11_000_0010_0010_001);
    /// Pointer Authentication Key B For Data (bits[63:0])
    pub const APDBKEYLO_EL1: Self = Self::System(0b11_000_0010_0010_010);
    /// Pointer Authentication Key B For Data (bits[127:64])
    pub const APDBKEYHI_EL1: Self = Self::System(0b11_000_0010_0010_011);
    /// Pointer Authentication Key A For Code (bits[63:0])
    pub const APGAKEYLO_EL1: Self = Self::System(0b11_000_0010_0011_000);
    /// Pointer Authentication Key A For Code (bits[127:64])
    pub const APGAKEYHI_EL1: Self = Self::System(0b11_000_0010_0011_001);
    /// Saved Program Status Register (EL1)
    pub const SPSR_EL1: Self = Self::System(0b11_000_0100_0000_000);
    /// Exception Link Register (EL1)
    pub const ELR_EL1: Self = Self::System(0b11_000_0100_0000_001);
    /// Stack Pointer (EL0)
    pub const SP_EL0: Self = Self::System(0b11_000_0100_0001_000);
    /// Interrupt Controller Interrupt Priority Mask Register
    pub const ICC_PMR_EL1: Self = Self::System(0b11_000_0100_0110_000);
    /// Interrupt Controller Virtual Interrupt Priority Mask Register
    pub const ICV_PMR_EL1: Self = Self::System(0b11_000_0100_0110_000);
    /// Auxiliary Fault Status Register 0 (EL1)
    pub const AFSR0_EL1: Self = Self::System(0b11_000_0101_0001_000);
    /// Auxiliary Fault Status Register 1 (EL1)
    pub const AFSR1_EL1: Self = Self::System(0b11_000_0101_0001_001);
    /// Exception Syndrome Register (EL1)
    pub const ESR_EL1: Self = Self::System(0b11_000_0101_0010_000);
    /// Error Record ID Register
    pub const ERRIDR_EL1: Self = Self::System(0b11_000_0101_0011_000);
    /// Error Record Select Register
    pub const ERRSELR_EL1: Self = Self::System(0b11_000_0101_0011_001);
    /// Selected Error Record Feature Register
    pub const ERXFR_EL1: Self = Self::System(0b11_000_0101_0100_000);
    /// Selected Error Record Control Register
    pub const ERXCTLR_EL1: Self = Self::System(0b11_000_0101_0100_001);
    /// Selected Error Record Primary Status Register
    pub const ERXSTATUS_EL1: Self = Self::System(0b11_000_0101_0100_010);
    /// Selected Error Record Address Register
    pub const ERXADDR_EL1: Self = Self::System(0b11_000_0101_0100_011);
    /// Selected Pseudo-fault Generation Feature Register
    pub const ERXPFGF_EL1: Self = Self::System(0b11_000_0101_0100_100);
    /// Selected Pseudo-fault Generation Control Register
    pub const ERXPFGCTL_EL1: Self = Self::System(0b11_000_0101_0100_101);
    /// Selected Pseudo-fault Generation Countdown Register
    pub const ERXPFGCDN_EL1: Self = Self::System(0b11_000_0101_0100_110);
    /// Selected Error Record Miscellaneous Register 0
    pub const ERXMISC0_EL1: Self = Self::System(0b11_000_0101_0101_000);
    /// Selected Error Record Miscellaneous Register 1
    pub const ERXMISC1_EL1: Self = Self::System(0b11_000_0101_0101_001);
    /// Selected Error Record Miscellaneous Register 2
    pub const ERXMISC2_EL1: Self = Self::System(0b11_000_0101_0101_010);
    /// Selected Error Record Miscellaneous Register 3
    pub const ERXMISC3_EL1: Self = Self::System(0b11_000_0101_0101_011);
    /// Tag Fault Status Register (EL1)
    pub const TFSR_EL1: Self = Self::System(0b11_000_0101_0110_000);
    /// Tag Fault Status Register (EL0)
    pub const TFSRE0_EL1: Self = Self::System(0b11_000_0101_0110_001);
    /// Fault Address Register (EL1)
    pub const FAR_EL1: Self = Self::System(0b11_000_0110_0000_000);
    /// Physical Address Register
    pub const PAR_EL1: Self = Self::System(0b11_000_0111_0100_000);
    /// Statistical Profiling Control Register (EL1)
    pub const PMSCR_EL1: Self = Self::System(0b11_000_1001_1001_000);
    /// Sampling Inverted Event Filter Register
    pub const PMSNEVFR_EL1: Self = Self::System(0b11_000_1001_1001_001);
    /// Sampling Interval Counter Register
    pub const PMSICR_EL1: Self = Self::System(0b11_000_1001_1001_010);
    /// Sampling Interval Reload Register
    pub const PMSIRR_EL1: Self = Self::System(0b11_000_1001_1001_011);
    /// Sampling Filter Control Register
    pub const PMSFCR_EL1: Self = Self::System(0b11_000_1001_1001_100);
    /// Sampling Event Filter Register
    pub const PMSEVFR_EL1: Self = Self::System(0b11_000_1001_1001_101);
    /// Sampling Latency Filter Register
    pub const PMSLATFR_EL1: Self = Self::System(0b11_000_1001_1001_110);
    /// Sampling Profiling ID Register
    pub const PMSIDR_EL1: Self = Self::System(0b11_000_1001_1001_111);
    /// Profiling Buffer Limit Address Register
    pub const PMBLIMITR_EL1: Self = Self::System(0b11_000_1001_1010_000);
    /// Profiling Buffer Write Pointer Register
    pub const PMBPTR_EL1: Self = Self::System(0b11_000_1001_1010_001);
    /// Profiling Buffer Status/syndrome Register
    pub const PMBSR_EL1: Self = Self::System(0b11_000_1001_1010_011);
    /// Profiling Buffer ID Register
    pub const PMBIDR_EL1: Self = Self::System(0b11_000_1001_1010_111);
    /// Trace Buffer Limit Address Register
    pub const TRBLIMITR_EL1: Self = Self::System(0b11_000_1001_1011_000);
    /// Trace Buffer Write Pointer Register
    pub const TRBPTR_EL1: Self = Self::System(0b11_000_1001_1011_001);
    /// Trace Buffer Base Address Register
    pub const TRBBASER_EL1: Self = Self::System(0b11_000_1001_1011_010);
    /// Trace Buffer Status/syndrome Register
    pub const TRBSR_EL1: Self = Self::System(0b11_000_1001_1011_011);
    /// Trace Buffer Memory Attribute Register
    pub const TRBMAR_EL1: Self = Self::System(0b11_000_1001_1011_100);
    /// Trace Buffer Trigger Counter Register
    pub const TRBTRG_EL1: Self = Self::System(0b11_000_1001_1011_110);
    /// Trace Buffer ID Register
    pub const TRBIDR_EL1: Self = Self::System(0b11_000_1001_1011_111);
    /// Performance Monitors Interrupt Enable Set Register
    pub const PMINTENSET_EL1: Self = Self::System(0b11_000_1001_1110_001);
    /// Performance Monitors Interrupt Enable Clear Register
    pub const PMINTENCLR_EL1: Self = Self::System(0b11_000_1001_1110_010);
    /// Performance Monitors Machine Identification Register
    pub const PMMIR_EL1: Self = Self::System(0b11_000_1001_1110_110);
    /// Memory Attribute Indirection Register (EL1)
    pub const MAIR_EL1: Self = Self::System(0b11_000_1010_0010_000);
    /// Auxiliary Memory Attribute Indirection Register (EL1)
    pub const AMAIR_EL1: Self = Self::System(0b11_000_1010_0011_000);
    /// LORegion Start Address (EL1)
    pub const LORSA_EL1: Self = Self::System(0b11_000_1010_0100_000);
    /// LORegion End Address (EL1)
    pub const LOREA_EL1: Self = Self::System(0b11_000_1010_0100_001);
    /// LORegion Number (EL1)
    pub const LORN_EL1: Self = Self::System(0b11_000_1010_0100_010);
    /// LORegion Control (EL1)
    pub const LORC_EL1: Self = Self::System(0b11_000_1010_0100_011);
    /// MPAM ID Register (EL1)
    pub const MPAMIDR_EL1: Self = Self::System(0b11_000_1010_0100_100);
    /// LORegionID (EL1)
    pub const LORID_EL1: Self = Self::System(0b11_000_1010_0100_111);
    /// MPAM1 Register (EL1)
    pub const MPAM1_EL1: Self = Self::System(0b11_000_1010_0101_000);
    /// MPAM0 Register (EL1)
    pub const MPAM0_EL1: Self = Self::System(0b11_000_1010_0101_001);
    /// MPAM Streaming Mode Register
    pub const MPAMSM_EL1: Self = Self::System(0b11_000_1010_0101_011);
    /// Vector Base Address Register (EL1)
    pub const VBAR_EL1: Self = Self::System(0b11_000_1100_0000_000);
    /// Reset Vector Base Address Register (if EL2 And EL3 Not Implemented)
    pub const RVBAR_EL1: Self = Self::System(0b11_000_1100_0000_001);
    /// Reset Management Register (EL1)
    pub const RMR_EL1: Self = Self::System(0b11_000_1100_0000_010);
    /// Interrupt Status Register
    pub const ISR_EL1: Self = Self::System(0b11_000_1100_0001_000);
    /// Deferred Interrupt Status Register
    pub const DISR_EL1: Self = Self::System(0b11_000_1100_0001_001);
    /// Interrupt Controller Interrupt Acknowledge Register 0
    pub const ICC_IAR0_EL1: Self = Self::System(0b11_000_1100_1000_000);
    /// Interrupt Controller Virtual Interrupt Acknowledge Register 0
    pub const ICV_IAR0_EL1: Self = Self::System(0b11_000_1100_1000_000);
    /// Interrupt Controller End Of Interrupt Register 0
    pub const ICC_EOIR0_EL1: Self = Self::System(0b11_000_1100_1000_001);
    /// Interrupt Controller Virtual End Of Interrupt Register 0
    pub const ICV_EOIR0_EL1: Self = Self::System(0b11_000_1100_1000_001);
    /// Interrupt Controller Highest Priority Pending Interrupt Register 0
    pub const ICC_HPPIR0_EL1: Self = Self::System(0b11_000_1100_1000_010);
    /// Interrupt Controller Virtual Highest Priority Pending Interrupt Register
    /// 0
    pub const ICV_HPPIR0_EL1: Self = Self::System(0b11_000_1100_1000_010);
    /// Interrupt Controller Binary Point Register 0
    pub const ICC_BPR0_EL1: Self = Self::System(0b11_000_1100_1000_011);
    /// Interrupt Controller Virtual Binary Point Register 0
    pub const ICV_BPR0_EL1: Self = Self::System(0b11_000_1100_1000_011);
    /// Interrupt Controller Active Priorities Group 0 Registers - 0
    pub const ICC_AP0R0_EL1: Self = Self::System(0b11_000_1100_1000_100);
    /// Interrupt Controller Virtual Active Priorities Group 0 Registers - 0
    pub const ICV_AP0R0_EL1: Self = Self::System(0b11_000_1100_1000_100);
    /// Interrupt Controller Active Priorities Group 0 Registers - 1
    pub const ICC_AP0R1_EL1: Self = Self::System(0b11_000_1100_1000_101);
    /// Interrupt Controller Virtual Active Priorities Group 0 Registers - 1
    pub const ICV_AP0R1_EL1: Self = Self::System(0b11_000_1100_1000_101);
    /// Interrupt Controller Active Priorities Group 0 Registers - 2
    pub const ICC_AP0R2_EL1: Self = Self::System(0b11_000_1100_1000_110);
    /// Interrupt Controller Virtual Active Priorities Group 0 Registers - 2
    pub const ICV_AP0R2_EL1: Self = Self::System(0b11_000_1100_1000_110);
    /// Interrupt Controller Active Priorities Group 0 Registers - 3
    pub const ICC_AP0R3_EL1: Self = Self::System(0b11_000_1100_1000_111);
    /// Interrupt Controller Virtual Active Priorities Group 0 Registers - 3
    pub const ICV_AP0R3_EL1: Self = Self::System(0b11_000_1100_1000_111);
    /// Interrupt Controller Active Priorities Group 1 Registers - 0
    pub const ICC_AP1R0_EL1: Self = Self::System(0b11_000_1100_1001_000);
    /// Interrupt Controller Virtual Active Priorities Group 1 Registers - 0
    pub const ICV_AP1R0_EL1: Self = Self::System(0b11_000_1100_1001_000);
    /// Interrupt Controller Active Priorities Group 1 Registers - 1
    pub const ICC_AP1R1_EL1: Self = Self::System(0b11_000_1100_1001_001);
    /// Interrupt Controller Virtual Active Priorities Group 1 Registers - 1
    pub const ICV_AP1R1_EL1: Self = Self::System(0b11_000_1100_1001_001);
    /// Interrupt Controller Active Priorities Group 1 Registers - 2
    pub const ICC_AP1R2_EL1: Self = Self::System(0b11_000_1100_1001_010);
    /// Interrupt Controller Virtual Active Priorities Group 1 Registers - 2
    pub const ICV_AP1R2_EL1: Self = Self::System(0b11_000_1100_1001_010);
    /// Interrupt Controller Active Priorities Group 1 Registers - 3
    pub const ICC_AP1R3_EL1: Self = Self::System(0b11_000_1100_1001_011);
    /// Interrupt Controller Virtual Active Priorities Group 1 Registers - 3
    pub const ICV_AP1R3_EL1: Self = Self::System(0b11_000_1100_1001_011);
    /// Interrupt Controller Non-maskable Interrupt Acknowledge Register 1
    pub const ICC_NMIAR1_EL1: Self = Self::System(0b11_000_1100_1001_101);
    /// Interrupt Controller Virtual Non-maskable Interrupt Acknowledge Register
    /// 1
    pub const ICV_NMIAR1_EL1: Self = Self::System(0b11_000_1100_1001_101);
    /// Interrupt Controller Deactivate Interrupt Register
    pub const ICC_DIR_EL1: Self = Self::System(0b11_000_1100_1011_001);
    /// Interrupt Controller Deactivate Virtual Interrupt Register
    pub const ICV_DIR_EL1: Self = Self::System(0b11_000_1100_1011_001);
    /// Interrupt Controller Running Priority Register
    pub const ICC_RPR_EL1: Self = Self::System(0b11_000_1100_1011_011);
    /// Interrupt Controller Virtual Running Priority Register
    pub const ICV_RPR_EL1: Self = Self::System(0b11_000_1100_1011_011);
    /// Interrupt Controller Software Generated Interrupt Group 1 Register
    pub const ICC_SGI1R_EL1: Self = Self::System(0b11_000_1100_1011_101);
    /// Interrupt Controller Alias Software Generated Interrupt Group 1 Register
    pub const ICC_ASGI1R_EL1: Self = Self::System(0b11_000_1100_1011_110);
    /// Interrupt Controller Software Generated Interrupt Group 0 Register
    pub const ICC_SGI0R_EL1: Self = Self::System(0b11_000_1100_1011_111);
    /// Interrupt Controller Interrupt Acknowledge Register 1
    pub const ICC_IAR1_EL1: Self = Self::System(0b11_000_1100_1100_000);
    /// Interrupt Controller Virtual Interrupt Acknowledge Register 1
    pub const ICV_IAR1_EL1: Self = Self::System(0b11_000_1100_1100_000);
    /// Interrupt Controller End Of Interrupt Register 1
    pub const ICC_EOIR1_EL1: Self = Self::System(0b11_000_1100_1100_001);
    /// Interrupt Controller Virtual End Of Interrupt Register 1
    pub const ICV_EOIR1_EL1: Self = Self::System(0b11_000_1100_1100_001);
    /// Interrupt Controller Highest Priority Pending Interrupt Register 1
    pub const ICC_HPPIR1_EL1: Self = Self::System(0b11_000_1100_1100_010);
    /// Interrupt Controller Virtual Highest Priority Pending Interrupt Register
    /// 1
    pub const ICV_HPPIR1_EL1: Self = Self::System(0b11_000_1100_1100_010);
    /// Interrupt Controller Binary Point Register 1
    pub const ICC_BPR1_EL1: Self = Self::System(0b11_000_1100_1100_011);
    /// Interrupt Controller Virtual Binary Point Register 1
    pub const ICV_BPR1_EL1: Self = Self::System(0b11_000_1100_1100_011);
    /// Interrupt Controller Control Register (EL1)
    pub const ICC_CTLR_EL1: Self = Self::System(0b11_000_1100_1100_100);
    /// Interrupt Controller Virtual Control Register
    pub const ICV_CTLR_EL1: Self = Self::System(0b11_000_1100_1100_100);
    /// Interrupt Controller System Register Enable Register (EL1)
    pub const ICC_SRE_EL1: Self = Self::System(0b11_000_1100_1100_101);
    /// Interrupt Controller Interrupt Group 0 Enable Register
    pub const ICC_IGRPEN0_EL1: Self = Self::System(0b11_000_1100_1100_110);
    /// Interrupt Controller Virtual Interrupt Group 0 Enable Register
    pub const ICV_IGRPEN0_EL1: Self = Self::System(0b11_000_1100_1100_110);
    /// Interrupt Controller Interrupt Group 1 Enable Register
    pub const ICC_IGRPEN1_EL1: Self = Self::System(0b11_000_1100_1100_111);
    /// Interrupt Controller Virtual Interrupt Group 1 Enable Register
    pub const ICV_IGRPEN1_EL1: Self = Self::System(0b11_000_1100_1100_111);
    /// Context ID Register (EL1)
    pub const CONTEXTIDR_EL1: Self = Self::System(0b11_000_1101_0000_001);
    /// EL1 Software Thread ID Register
    pub const TPIDR_EL1: Self = Self::System(0b11_000_1101_0000_100);
    /// Accelerator Data
    pub const ACCDATA_EL1: Self = Self::System(0b11_000_1101_0000_101);
    /// EL1 Read/Write Software Context Number
    pub const SCXTNUM_EL1: Self = Self::System(0b11_000_1101_0000_111);
    /// Counter-timer Kernel Control Register
    pub const CNTKCTL_EL1: Self = Self::System(0b11_000_1110_0001_000);
    /// Current Cache Size ID Register
    pub const CCSIDR_EL1: Self = Self::System(0b11_001_0000_0000_000);
    /// Cache Level ID Register
    pub const CLIDR_EL1: Self = Self::System(0b11_001_0000_0000_001);
    /// Current Cache Size ID Register 2
    pub const CCSIDR2_EL1: Self = Self::System(0b11_001_0000_0000_010);
    /// Multiple Tag Transfer ID Register
    pub const GMID_EL1: Self = Self::System(0b11_001_0000_0000_100);
    /// Streaming Mode Identification Register
    pub const SMIDR_EL1: Self = Self::System(0b11_001_0000_0000_110);
    /// Auxiliary ID Register
    pub const AIDR_EL1: Self = Self::System(0b11_001_0000_0000_111);
    /// Cache Size Selection Register
    pub const CSSELR_EL1: Self = Self::System(0b11_010_0000_0000_000);
    /// Cache Type Register
    pub const CTR_EL0: Self = Self::System(0b11_011_0000_0000_001);
    /// Data Cache Zero ID Register
    pub const DCZID_EL0: Self = Self::System(0b11_011_0000_0000_111);
    /// Random Number
    pub const RNDR: Self = Self::System(0b11_011_0010_0100_000);
    /// Reseeded Random Number
    pub const RNDRRS: Self = Self::System(0b11_011_0010_0100_001);
    /// Streaming Vector Control Register
    pub const SVCR: Self = Self::System(0b11_011_0100_0010_010);
    /// Floating-point Control Register
    pub const FPCR: Self = Self::System(0b11_011_0100_0100_000);
    /// Floating-point Status Register
    pub const FPSR: Self = Self::System(0b11_011_0100_0100_001);
    /// Debug Saved Program Status Register
    pub const DSPSR_EL0: Self = Self::System(0b11_011_0100_0101_000);
    /// Debug Link Register
    pub const DLR_EL0: Self = Self::System(0b11_011_0100_0101_001);
    /// Performance Monitors Control Register
    pub const PMCR_EL0: Self = Self::System(0b11_011_1001_1100_000);
    /// Performance Monitors Count Enable Set Register
    pub const PMCNTENSET_EL0: Self = Self::System(0b11_011_1001_1100_001);
    /// Performance Monitors Count Enable Clear Register
    pub const PMCNTENCLR_EL0: Self = Self::System(0b11_011_1001_1100_010);
    /// Performance Monitors Overflow Flag Status Clear Register
    pub const PMOVSCLR_EL0: Self = Self::System(0b11_011_1001_1100_011);
    /// Performance Monitors Software Increment Register
    pub const PMSWINC_EL0: Self = Self::System(0b11_011_1001_1100_100);
    /// Performance Monitors Event Counter Selection Register
    pub const PMSELR_EL0: Self = Self::System(0b11_011_1001_1100_101);
    /// Performance Monitors Common Event Identification Register 0
    pub const PMCEID0_EL0: Self = Self::System(0b11_011_1001_1100_110);
    /// Performance Monitors Common Event Identification Register 1
    pub const PMCEID1_EL0: Self = Self::System(0b11_011_1001_1100_111);
    /// Performance Monitors Cycle Count Register
    pub const PMCCNTR_EL0: Self = Self::System(0b11_011_1001_1101_000);
    /// Performance Monitors Selected Event Type Register
    pub const PMXEVTYPER_EL0: Self = Self::System(0b11_011_1001_1101_001);
    /// Performance Monitors Selected Event Count Register
    pub const PMXEVCNTR_EL0: Self = Self::System(0b11_011_1001_1101_010);
    /// Performance Monitors User Enable Register
    pub const PMUSERENR_EL0: Self = Self::System(0b11_011_1001_1110_000);
    /// Performance Monitors Overflow Flag Status Set Register
    pub const PMOVSSET_EL0: Self = Self::System(0b11_011_1001_1110_011);
    /// EL0 Read/Write Software Thread ID Register
    pub const TPIDR_EL0: Self = Self::System(0b11_011_1101_0000_010);
    /// EL0 Read-Only Software Thread ID Register
    pub const TPIDRRO_EL0: Self = Self::System(0b11_011_1101_0000_011);
    /// EL0 Read/Write Software Thread ID Register 2
    pub const TPIDR2_EL0: Self = Self::System(0b11_011_1101_0000_101);
    /// EL0 Read/Write Software Context Number
    pub const SCXTNUM_EL0: Self = Self::System(0b11_011_1101_0000_111);
    /// Activity Monitors Control Register
    pub const AMCR_EL0: Self = Self::System(0b11_011_1101_0010_000);
    /// Activity Monitors Configuration Register
    pub const AMCFGR_EL0: Self = Self::System(0b11_011_1101_0010_001);
    /// Activity Monitors Counter Group Configuration Register
    pub const AMCGCR_EL0: Self = Self::System(0b11_011_1101_0010_010);
    /// Activity Monitors User Enable Register
    pub const AMUSERENR_EL0: Self = Self::System(0b11_011_1101_0010_011);
    /// Activity Monitors Count Enable Clear Register 0
    pub const AMCNTENCLR0_EL0: Self = Self::System(0b11_011_1101_0010_100);
    /// Activity Monitors Count Enable Set Register 0
    pub const AMCNTENSET0_EL0: Self = Self::System(0b11_011_1101_0010_101);
    /// Activity Monitors Counter Group 1 Identification Register
    pub const AMCG1IDR_EL0: Self = Self::System(0b11_011_1101_0010_110);
    /// Activity Monitors Count Enable Clear Register 1
    pub const AMCNTENCLR1_EL0: Self = Self::System(0b11_011_1101_0011_000);
    /// Activity Monitors Count Enable Set Register 1
    pub const AMCNTENSET1_EL0: Self = Self::System(0b11_011_1101_0011_001);
    /// Activity Monitors Event Counter Registers 0 - 0
    pub const AMEVCNTR00_EL0: Self = Self::System(0b11_011_1101_0100_000);
    /// Activity Monitors Event Counter Registers 0 - 1
    pub const AMEVCNTR01_EL0: Self = Self::System(0b11_011_1101_0100_001);
    /// Activity Monitors Event Counter Registers 0 - 2
    pub const AMEVCNTR02_EL0: Self = Self::System(0b11_011_1101_0100_010);
    /// Activity Monitors Event Counter Registers 0 - 3
    pub const AMEVCNTR03_EL0: Self = Self::System(0b11_011_1101_0100_011);
    /// Activity Monitors Event Type Registers 0 - 0
    pub const AMEVTYPER00_EL0: Self = Self::System(0b11_011_1101_0110_000);
    /// Activity Monitors Event Type Registers 0 - 1
    pub const AMEVTYPER01_EL0: Self = Self::System(0b11_011_1101_0110_001);
    /// Activity Monitors Event Type Registers 0 - 2
    pub const AMEVTYPER02_EL0: Self = Self::System(0b11_011_1101_0110_010);
    /// Activity Monitors Event Type Registers 0 - 3
    pub const AMEVTYPER03_EL0: Self = Self::System(0b11_011_1101_0110_011);
    /// Activity Monitors Event Counter Registers 1 - 0
    pub const AMEVCNTR10_EL0: Self = Self::System(0b11_011_1101_1100_000);
    /// Activity Monitors Event Counter Registers 1 - 1
    pub const AMEVCNTR11_EL0: Self = Self::System(0b11_011_1101_1100_001);
    /// Activity Monitors Event Counter Registers 1 - 2
    pub const AMEVCNTR12_EL0: Self = Self::System(0b11_011_1101_1100_010);
    /// Activity Monitors Event Counter Registers 1 - 3
    pub const AMEVCNTR13_EL0: Self = Self::System(0b11_011_1101_1100_011);
    /// Activity Monitors Event Counter Registers 1 - 4
    pub const AMEVCNTR14_EL0: Self = Self::System(0b11_011_1101_1100_100);
    /// Activity Monitors Event Counter Registers 1 - 5
    pub const AMEVCNTR15_EL0: Self = Self::System(0b11_011_1101_1100_101);
    /// Activity Monitors Event Counter Registers 1 - 6
    pub const AMEVCNTR16_EL0: Self = Self::System(0b11_011_1101_1100_110);
    /// Activity Monitors Event Counter Registers 1 - 7
    pub const AMEVCNTR17_EL0: Self = Self::System(0b11_011_1101_1100_111);
    /// Activity Monitors Event Counter Registers 1 - 8
    pub const AMEVCNTR18_EL0: Self = Self::System(0b11_011_1101_1101_000);
    /// Activity Monitors Event Counter Registers 1 - 9
    pub const AMEVCNTR19_EL0: Self = Self::System(0b11_011_1101_1101_001);
    /// Activity Monitors Event Counter Registers 1 - 10
    pub const AMEVCNTR110_EL0: Self = Self::System(0b11_011_1101_1101_010);
    /// Activity Monitors Event Counter Registers 1 - 11
    pub const AMEVCNTR111_EL0: Self = Self::System(0b11_011_1101_1101_011);
    /// Activity Monitors Event Counter Registers 1 - 12
    pub const AMEVCNTR112_EL0: Self = Self::System(0b11_011_1101_1101_100);
    /// Activity Monitors Event Counter Registers 1 - 13
    pub const AMEVCNTR113_EL0: Self = Self::System(0b11_011_1101_1101_101);
    /// Activity Monitors Event Counter Registers 1 - 14
    pub const AMEVCNTR114_EL0: Self = Self::System(0b11_011_1101_1101_110);
    /// Activity Monitors Event Counter Registers 1 - 15
    pub const AMEVCNTR115_EL0: Self = Self::System(0b11_011_1101_1101_111);
    /// Activity Monitors Event Type Registers 1 - 0
    pub const AMEVTYPER10_EL0: Self = Self::System(0b11_011_1101_1110_000);
    /// Activity Monitors Event Type Registers 1 - 1
    pub const AMEVTYPER11_EL0: Self = Self::System(0b11_011_1101_1110_001);
    /// Activity Monitors Event Type Registers 1 - 2
    pub const AMEVTYPER12_EL0: Self = Self::System(0b11_011_1101_1110_010);
    /// Activity Monitors Event Type Registers 1 - 3
    pub const AMEVTYPER13_EL0: Self = Self::System(0b11_011_1101_1110_011);
    /// Activity Monitors Event Type Registers 1 - 4
    pub const AMEVTYPER14_EL0: Self = Self::System(0b11_011_1101_1110_100);
    /// Activity Monitors Event Type Registers 1 - 5
    pub const AMEVTYPER15_EL0: Self = Self::System(0b11_011_1101_1110_101);
    /// Activity Monitors Event Type Registers 1 - 6
    pub const AMEVTYPER16_EL0: Self = Self::System(0b11_011_1101_1110_110);
    /// Activity Monitors Event Type Registers 1 - 7
    pub const AMEVTYPER17_EL0: Self = Self::System(0b11_011_1101_1110_111);
    /// Activity Monitors Event Type Registers 1 - 8
    pub const AMEVTYPER18_EL0: Self = Self::System(0b11_011_1101_1111_000);
    /// Activity Monitors Event Type Registers 1 - 9
    pub const AMEVTYPER19_EL0: Self = Self::System(0b11_011_1101_1111_001);
    /// Activity Monitors Event Type Registers 1 - 10
    pub const AMEVTYPER110_EL0: Self = Self::System(0b11_011_1101_1111_010);
    /// Activity Monitors Event Type Registers 1 - 11
    pub const AMEVTYPER111_EL0: Self = Self::System(0b11_011_1101_1111_011);
    /// Activity Monitors Event Type Registers 1 - 12
    pub const AMEVTYPER112_EL0: Self = Self::System(0b11_011_1101_1111_100);
    /// Activity Monitors Event Type Registers 1 - 13
    pub const AMEVTYPER113_EL0: Self = Self::System(0b11_011_1101_1111_101);
    /// Activity Monitors Event Type Registers 1 - 14
    pub const AMEVTYPER114_EL0: Self = Self::System(0b11_011_1101_1111_110);
    /// Activity Monitors Event Type Registers 1 - 15
    pub const AMEVTYPER115_EL0: Self = Self::System(0b11_011_1101_1111_111);
    /// Counter-timer Frequency Register
    pub const CNTFRQ_EL0: Self = Self::System(0b11_011_1110_0000_000);
    /// Counter-timer Physical Count Register
    pub const CNTPCT_EL0: Self = Self::System(0b11_011_1110_0000_001);
    /// Counter-timer Virtual Count Register
    pub const CNTVCT_EL0: Self = Self::System(0b11_011_1110_0000_010);
    /// Counter-timer Self-Synchronized Physical Count Register
    pub const CNTPCTSS_EL0: Self = Self::System(0b11_011_1110_0000_101);
    /// Counter-timer Self-Synchronized Virtual Count Register
    pub const CNTVCTSS_EL0: Self = Self::System(0b11_011_1110_0000_110);
    /// Counter-timer Physical Timer TimerValue Register
    pub const CNTP_TVAL_EL0: Self = Self::System(0b11_011_1110_0010_000);
    /// Counter-timer Physical Timer Control Register
    pub const CNTP_CTL_EL0: Self = Self::System(0b11_011_1110_0010_001);
    /// Counter-timer Physical Timer CompareValue Register
    pub const CNTP_CVAL_EL0: Self = Self::System(0b11_011_1110_0010_010);
    /// Counter-timer Virtual Timer TimerValue Register
    pub const CNTV_TVAL_EL0: Self = Self::System(0b11_011_1110_0011_000);
    /// Counter-timer Virtual Timer Control Register
    pub const CNTV_CTL_EL0: Self = Self::System(0b11_011_1110_0011_001);
    /// Counter-timer Virtual Timer CompareValue Register
    pub const CNTV_CVAL_EL0: Self = Self::System(0b11_011_1110_0011_010);
    /// Performance Monitors Event Count Registers - 0
    pub const PMEVCNTR0_EL0: Self = Self::System(0b11_011_1110_1000_000);
    /// Performance Monitors Event Count Registers - 1
    pub const PMEVCNTR1_EL0: Self = Self::System(0b11_011_1110_1000_001);
    /// Performance Monitors Event Count Registers - 2
    pub const PMEVCNTR2_EL0: Self = Self::System(0b11_011_1110_1000_010);
    /// Performance Monitors Event Count Registers - 3
    pub const PMEVCNTR3_EL0: Self = Self::System(0b11_011_1110_1000_011);
    /// Performance Monitors Event Count Registers - 4
    pub const PMEVCNTR4_EL0: Self = Self::System(0b11_011_1110_1000_100);
    /// Performance Monitors Event Count Registers - 5
    pub const PMEVCNTR5_EL0: Self = Self::System(0b11_011_1110_1000_101);
    /// Performance Monitors Event Count Registers - 6
    pub const PMEVCNTR6_EL0: Self = Self::System(0b11_011_1110_1000_110);
    /// Performance Monitors Event Count Registers - 7
    pub const PMEVCNTR7_EL0: Self = Self::System(0b11_011_1110_1000_111);
    /// Performance Monitors Event Count Registers - 8
    pub const PMEVCNTR8_EL0: Self = Self::System(0b11_011_1110_1001_000);
    /// Performance Monitors Event Count Registers - 9
    pub const PMEVCNTR9_EL0: Self = Self::System(0b11_011_1110_1001_001);
    /// Performance Monitors Event Count Registers - 10
    pub const PMEVCNTR10_EL0: Self = Self::System(0b11_011_1110_1001_010);
    /// Performance Monitors Event Count Registers - 11
    pub const PMEVCNTR11_EL0: Self = Self::System(0b11_011_1110_1001_011);
    /// Performance Monitors Event Count Registers - 12
    pub const PMEVCNTR12_EL0: Self = Self::System(0b11_011_1110_1001_100);
    /// Performance Monitors Event Count Registers - 13
    pub const PMEVCNTR13_EL0: Self = Self::System(0b11_011_1110_1001_101);
    /// Performance Monitors Event Count Registers - 14
    pub const PMEVCNTR14_EL0: Self = Self::System(0b11_011_1110_1001_110);
    /// Performance Monitors Event Count Registers - 15
    pub const PMEVCNTR15_EL0: Self = Self::System(0b11_011_1110_1001_111);
    /// Performance Monitors Event Count Registers - 16
    pub const PMEVCNTR16_EL0: Self = Self::System(0b11_011_1110_1010_000);
    /// Performance Monitors Event Count Registers - 17
    pub const PMEVCNTR17_EL0: Self = Self::System(0b11_011_1110_1010_001);
    /// Performance Monitors Event Count Registers - 18
    pub const PMEVCNTR18_EL0: Self = Self::System(0b11_011_1110_1010_010);
    /// Performance Monitors Event Count Registers - 19
    pub const PMEVCNTR19_EL0: Self = Self::System(0b11_011_1110_1010_011);
    /// Performance Monitors Event Count Registers - 20
    pub const PMEVCNTR20_EL0: Self = Self::System(0b11_011_1110_1010_100);
    /// Performance Monitors Event Count Registers - 21
    pub const PMEVCNTR21_EL0: Self = Self::System(0b11_011_1110_1010_101);
    /// Performance Monitors Event Count Registers - 22
    pub const PMEVCNTR22_EL0: Self = Self::System(0b11_011_1110_1010_110);
    /// Performance Monitors Event Count Registers - 23
    pub const PMEVCNTR23_EL0: Self = Self::System(0b11_011_1110_1010_111);
    /// Performance Monitors Event Count Registers - 24
    pub const PMEVCNTR24_EL0: Self = Self::System(0b11_011_1110_1011_000);
    /// Performance Monitors Event Count Registers - 25
    pub const PMEVCNTR25_EL0: Self = Self::System(0b11_011_1110_1011_001);
    /// Performance Monitors Event Count Registers - 26
    pub const PMEVCNTR26_EL0: Self = Self::System(0b11_011_1110_1011_010);
    /// Performance Monitors Event Count Registers - 27
    pub const PMEVCNTR27_EL0: Self = Self::System(0b11_011_1110_1011_011);
    /// Performance Monitors Event Count Registers - 28
    pub const PMEVCNTR28_EL0: Self = Self::System(0b11_011_1110_1011_100);
    /// Performance Monitors Event Count Registers - 29
    pub const PMEVCNTR29_EL0: Self = Self::System(0b11_011_1110_1011_101);
    /// Performance Monitors Event Count Registers - 30
    pub const PMEVCNTR30_EL0: Self = Self::System(0b11_011_1110_1011_110);
    /// Performance Monitors Event Type Registers - 0
    pub const PMEVTYPER0_EL0: Self = Self::System(0b11_011_1110_1100_000);
    /// Performance Monitors Event Type Registers - 1
    pub const PMEVTYPER1_EL0: Self = Self::System(0b11_011_1110_1100_001);
    /// Performance Monitors Event Type Registers - 2
    pub const PMEVTYPER2_EL0: Self = Self::System(0b11_011_1110_1100_010);
    /// Performance Monitors Event Type Registers - 3
    pub const PMEVTYPER3_EL0: Self = Self::System(0b11_011_1110_1100_011);
    /// Performance Monitors Event Type Registers - 4
    pub const PMEVTYPER4_EL0: Self = Self::System(0b11_011_1110_1100_100);
    /// Performance Monitors Event Type Registers - 5
    pub const PMEVTYPER5_EL0: Self = Self::System(0b11_011_1110_1100_101);
    /// Performance Monitors Event Type Registers - 6
    pub const PMEVTYPER6_EL0: Self = Self::System(0b11_011_1110_1100_110);
    /// Performance Monitors Event Type Registers - 7
    pub const PMEVTYPER7_EL0: Self = Self::System(0b11_011_1110_1100_111);
    /// Performance Monitors Event Type Registers - 8
    pub const PMEVTYPER8_EL0: Self = Self::System(0b11_011_1110_1101_000);
    /// Performance Monitors Event Type Registers - 9
    pub const PMEVTYPER9_EL0: Self = Self::System(0b11_011_1110_1101_001);
    /// Performance Monitors Event Type Registers - 10
    pub const PMEVTYPER10_EL0: Self = Self::System(0b11_011_1110_1101_010);
    /// Performance Monitors Event Type Registers - 11
    pub const PMEVTYPER11_EL0: Self = Self::System(0b11_011_1110_1101_011);
    /// Performance Monitors Event Type Registers - 12
    pub const PMEVTYPER12_EL0: Self = Self::System(0b11_011_1110_1101_100);
    /// Performance Monitors Event Type Registers - 13
    pub const PMEVTYPER13_EL0: Self = Self::System(0b11_011_1110_1101_101);
    /// Performance Monitors Event Type Registers - 14
    pub const PMEVTYPER14_EL0: Self = Self::System(0b11_011_1110_1101_110);
    /// Performance Monitors Event Type Registers - 15
    pub const PMEVTYPER15_EL0: Self = Self::System(0b11_011_1110_1101_111);
    /// Performance Monitors Event Type Registers - 16
    pub const PMEVTYPER16_EL0: Self = Self::System(0b11_011_1110_1110_000);
    /// Performance Monitors Event Type Registers - 17
    pub const PMEVTYPER17_EL0: Self = Self::System(0b11_011_1110_1110_001);
    /// Performance Monitors Event Type Registers - 18
    pub const PMEVTYPER18_EL0: Self = Self::System(0b11_011_1110_1110_010);
    /// Performance Monitors Event Type Registers - 19
    pub const PMEVTYPER19_EL0: Self = Self::System(0b11_011_1110_1110_011);
    /// Performance Monitors Event Type Registers - 20
    pub const PMEVTYPER20_EL0: Self = Self::System(0b11_011_1110_1110_100);
    /// Performance Monitors Event Type Registers - 21
    pub const PMEVTYPER21_EL0: Self = Self::System(0b11_011_1110_1110_101);
    /// Performance Monitors Event Type Registers - 22
    pub const PMEVTYPER22_EL0: Self = Self::System(0b11_011_1110_1110_110);
    /// Performance Monitors Event Type Registers - 23
    pub const PMEVTYPER23_EL0: Self = Self::System(0b11_011_1110_1110_111);
    /// Performance Monitors Event Type Registers - 24
    pub const PMEVTYPER24_EL0: Self = Self::System(0b11_011_1110_1111_000);
    /// Performance Monitors Event Type Registers - 25
    pub const PMEVTYPER25_EL0: Self = Self::System(0b11_011_1110_1111_001);
    /// Performance Monitors Event Type Registers - 26
    pub const PMEVTYPER26_EL0: Self = Self::System(0b11_011_1110_1111_010);
    /// Performance Monitors Event Type Registers - 27
    pub const PMEVTYPER27_EL0: Self = Self::System(0b11_011_1110_1111_011);
    /// Performance Monitors Event Type Registers - 28
    pub const PMEVTYPER28_EL0: Self = Self::System(0b11_011_1110_1111_100);
    /// Performance Monitors Event Type Registers - 29
    pub const PMEVTYPER29_EL0: Self = Self::System(0b11_011_1110_1111_101);
    /// Performance Monitors Event Type Registers - 30
    pub const PMEVTYPER30_EL0: Self = Self::System(0b11_011_1110_1111_110);
    /// Performance Monitors Cycle Count Filter Register
    pub const PMCCFILTR_EL0: Self = Self::System(0b11_011_1110_1111_111);
    /// Virtualization Processor ID Register
    pub const VPIDR_EL2: Self = Self::System(0b11_100_0000_0000_000);
    /// Virtualization Multiprocessor ID Register
    pub const VMPIDR_EL2: Self = Self::System(0b11_100_0000_0000_101);
    /// System Control Register (EL2)
    pub const SCTLR_EL2: Self = Self::System(0b11_100_0001_0000_000);
    /// Auxiliary Control Register (EL2)
    pub const ACTLR_EL2: Self = Self::System(0b11_100_0001_0000_001);
    /// Hypervisor Configuration Register
    pub const HCR_EL2: Self = Self::System(0b11_100_0001_0001_000);
    /// Monitor Debug Configuration Register (EL2)
    pub const MDCR_EL2: Self = Self::System(0b11_100_0001_0001_001);
    /// Architectural Feature Trap Register (EL2)
    pub const CPTR_EL2: Self = Self::System(0b11_100_0001_0001_010);
    /// Hypervisor System Trap Register
    pub const HSTR_EL2: Self = Self::System(0b11_100_0001_0001_011);
    /// Hypervisor Fine-Grained Read Trap Register
    pub const HFGRTR_EL2: Self = Self::System(0b11_100_0001_0001_100);
    /// Hypervisor Fine-Grained Write Trap Register
    pub const HFGWTR_EL2: Self = Self::System(0b11_100_0001_0001_101);
    /// Hypervisor Fine-Grained Instruction Trap Register
    pub const HFGITR_EL2: Self = Self::System(0b11_100_0001_0001_110);
    /// Hypervisor Auxiliary Control Register
    pub const HACR_EL2: Self = Self::System(0b11_100_0001_0001_111);
    /// SVE Control Register (EL2)
    pub const ZCR_EL2: Self = Self::System(0b11_100_0001_0010_000);
    /// Trace Filter Control Register (EL2)
    pub const TRFCR_EL2: Self = Self::System(0b11_100_0001_0010_001);
    /// Extended Hypervisor Configuration Register
    pub const HCRX_EL2: Self = Self::System(0b11_100_0001_0010_010);
    /// Streaming Mode Priority Mapping Register
    pub const SMPRIMAP_EL2: Self = Self::System(0b11_100_0001_0010_101);
    /// SME Control Register (EL2)
    pub const SMCR_EL2: Self = Self::System(0b11_100_0001_0010_110);
    /// AArch32 Secure Debug Enable Register
    pub const SDER32_EL2: Self = Self::System(0b11_100_0001_0011_001);
    /// Translation Table Base Register 0 (EL2)
    pub const TTBR0_EL2: Self = Self::System(0b11_100_0010_0000_000);
    /// Translation Table Base Register 1 (EL2)
    pub const TTBR1_EL2: Self = Self::System(0b11_100_0010_0000_001);
    /// Translation Control Register (EL2)
    pub const TCR_EL2: Self = Self::System(0b11_100_0010_0000_010);
    /// Virtualization Translation Table Base Register
    pub const VTTBR_EL2: Self = Self::System(0b11_100_0010_0001_000);
    /// Virtualization Translation Control Register
    pub const VTCR_EL2: Self = Self::System(0b11_100_0010_0001_010);
    /// Virtual Nested Control Register
    pub const VNCR_EL2: Self = Self::System(0b11_100_0010_0010_000);
    /// Virtualization Secure Translation Table Base Register
    pub const VSTTBR_EL2: Self = Self::System(0b11_100_0010_0110_000);
    /// Virtualization Secure Translation Control Register
    pub const VSTCR_EL2: Self = Self::System(0b11_100_0010_0110_010);
    /// Domain Access Control Register
    pub const DACR32_EL2: Self = Self::System(0b11_100_0011_0000_000);
    /// Hypervisor Debug Fine-Grained Read Trap Register
    pub const HDFGRTR_EL2: Self = Self::System(0b11_100_0011_0001_100);
    /// Hypervisor Debug Fine-Grained Write Trap Register
    pub const HDFGWTR_EL2: Self = Self::System(0b11_100_0011_0001_101);
    /// Hypervisor Activity Monitors Fine-Grained Read Trap Register
    pub const HAFGRTR_EL2: Self = Self::System(0b11_100_0011_0001_110);
    /// Saved Program Status Register (EL2)
    pub const SPSR_EL2: Self = Self::System(0b11_100_0100_0000_000);
    /// Exception Link Register (EL2)
    pub const ELR_EL2: Self = Self::System(0b11_100_0100_0000_001);
    /// Stack Pointer (EL1)
    pub const SP_EL1: Self = Self::System(0b11_100_0100_0001_000);
    /// Saved Program Status Register (IRQ Mode)
    pub const SPSR_IRQ: Self = Self::System(0b11_100_0100_0011_000);
    /// Saved Program Status Register (Abort Mode)
    pub const SPSR_ABT: Self = Self::System(0b11_100_0100_0011_001);
    /// Saved Program Status Register (Undefined Mode)
    pub const SPSR_UND: Self = Self::System(0b11_100_0100_0011_010);
    /// Saved Program Status Register (FIQ Mode)
    pub const SPSR_FIQ: Self = Self::System(0b11_100_0100_0011_011);
    /// Instruction Fault Status Register (EL2)
    pub const IFSR32_EL2: Self = Self::System(0b11_100_0101_0000_001);
    /// Auxiliary Fault Status Register 0 (EL2)
    pub const AFSR0_EL2: Self = Self::System(0b11_100_0101_0001_000);
    /// Auxiliary Fault Status Register 1 (EL2)
    pub const AFSR1_EL2: Self = Self::System(0b11_100_0101_0001_001);
    /// Exception Syndrome Register (EL2)
    pub const ESR_EL2: Self = Self::System(0b11_100_0101_0010_000);
    /// Virtual SError Exception Syndrome Register
    pub const VSESR_EL2: Self = Self::System(0b11_100_0101_0010_011);
    /// Floating-Point Exception Control Register
    pub const FPEXC32_EL2: Self = Self::System(0b11_100_0101_0011_000);
    /// Tag Fault Status Register (EL2)
    pub const TFSR_EL2: Self = Self::System(0b11_100_0101_0110_000);
    /// Fault Address Register (EL2)
    pub const FAR_EL2: Self = Self::System(0b11_100_0110_0000_000);
    /// Hypervisor IPA Fault Address Register
    pub const HPFAR_EL2: Self = Self::System(0b11_100_0110_0000_100);
    /// Statistical Profiling Control Register (EL2)
    pub const PMSCR_EL2: Self = Self::System(0b11_100_1001_1001_000);
    /// Memory Attribute Indirection Register (EL2)
    pub const MAIR_EL2: Self = Self::System(0b11_100_1010_0010_000);
    /// Auxiliary Memory Attribute Indirection Register (EL2)
    pub const AMAIR_EL2: Self = Self::System(0b11_100_1010_0011_000);
    /// MPAM Hypervisor Control Register (EL2)
    pub const MPAMHCR_EL2: Self = Self::System(0b11_100_1010_0100_000);
    /// MPAM Virtual Partition Mapping Valid Register
    pub const MPAMVPMV_EL2: Self = Self::System(0b11_100_1010_0100_001);
    /// MPAM2 Register (EL2)
    pub const MPAM2_EL2: Self = Self::System(0b11_100_1010_0101_000);
    /// MPAM Virtual PARTID Mapping Register 0
    pub const MPAMVPM0_EL2: Self = Self::System(0b11_100_1010_0110_000);
    /// MPAM Virtual PARTID Mapping Register 1
    pub const MPAMVPM1_EL2: Self = Self::System(0b11_100_1010_0110_001);
    /// MPAM Virtual PARTID Mapping Register 2
    pub const MPAMVPM2_EL2: Self = Self::System(0b11_100_1010_0110_010);
    /// MPAM Virtual PARTID Mapping Register 3
    pub const MPAMVPM3_EL2: Self = Self::System(0b11_100_1010_0110_011);
    /// MPAM Virtual PARTID Mapping Register 4
    pub const MPAMVPM4_EL2: Self = Self::System(0b11_100_1010_0110_100);
    /// MPAM Virtual PARTID Mapping Register 5
    pub const MPAMVPM5_EL2: Self = Self::System(0b11_100_1010_0110_101);
    /// MPAM Virtual PARTID Mapping Register 6
    pub const MPAMVPM6_EL2: Self = Self::System(0b11_100_1010_0110_110);
    /// MPAM Virtual PARTID Mapping Register 7
    pub const MPAMVPM7_EL2: Self = Self::System(0b11_100_1010_0110_111);
    /// Vector Base Address Register (EL2)
    pub const VBAR_EL2: Self = Self::System(0b11_100_1100_0000_000);
    /// Reset Vector Base Address Register (if EL3 Not Implemented)
    pub const RVBAR_EL2: Self = Self::System(0b11_100_1100_0000_001);
    /// Reset Management Register (EL2)
    pub const RMR_EL2: Self = Self::System(0b11_100_1100_0000_010);
    /// Virtual Deferred Interrupt Status Register
    pub const VDISR_EL2: Self = Self::System(0b11_100_1100_0001_001);
    /// Interrupt Controller Hyp Active Priorities Group 0 Registers - 0
    pub const ICH_AP0R0_EL2: Self = Self::System(0b11_100_1100_1000_000);
    /// Interrupt Controller Hyp Active Priorities Group 0 Registers - 1
    pub const ICH_AP0R1_EL2: Self = Self::System(0b11_100_1100_1000_001);
    /// Interrupt Controller Hyp Active Priorities Group 0 Registers - 2
    pub const ICH_AP0R2_EL2: Self = Self::System(0b11_100_1100_1000_010);
    /// Interrupt Controller Hyp Active Priorities Group 0 Registers - 3
    pub const ICH_AP0R3_EL2: Self = Self::System(0b11_100_1100_1000_011);
    /// Interrupt Controller Hyp Active Priorities Group 1 Registers - 0
    pub const ICH_AP1R0_EL2: Self = Self::System(0b11_100_1100_1001_000);
    /// Interrupt Controller Hyp Active Priorities Group 1 Registers - 1
    pub const ICH_AP1R1_EL2: Self = Self::System(0b11_100_1100_1001_001);
    /// Interrupt Controller Hyp Active Priorities Group 1 Registers - 2
    pub const ICH_AP1R2_EL2: Self = Self::System(0b11_100_1100_1001_010);
    /// Interrupt Controller Hyp Active Priorities Group 1 Registers - 3
    pub const ICH_AP1R3_EL2: Self = Self::System(0b11_100_1100_1001_011);
    /// Interrupt Controller System Register Enable Register (EL2)
    pub const ICC_SRE_EL2: Self = Self::System(0b11_100_1100_1001_101);
    /// Interrupt Controller Hyp Control Register
    pub const ICH_HCR_EL2: Self = Self::System(0b11_100_1100_1011_000);
    /// Interrupt Controller VGIC Type Register
    pub const ICH_VTR_EL2: Self = Self::System(0b11_100_1100_1011_001);
    /// Interrupt Controller Maintenance Interrupt State Register
    pub const ICH_MISR_EL2: Self = Self::System(0b11_100_1100_1011_010);
    /// Interrupt Controller End Of Interrupt Status Register
    pub const ICH_EISR_EL2: Self = Self::System(0b11_100_1100_1011_011);
    /// Interrupt Controller Empty List Register Status Register
    pub const ICH_ELRSR_EL2: Self = Self::System(0b11_100_1100_1011_101);
    /// Interrupt Controller Virtual Machine Control Register
    pub const ICH_VMCR_EL2: Self = Self::System(0b11_100_1100_1011_111);
    /// Interrupt Controller List Registers - 0
    pub const ICH_LR0_EL2: Self = Self::System(0b11_100_1100_1100_000);
    /// Interrupt Controller List Registers - 1
    pub const ICH_LR1_EL2: Self = Self::System(0b11_100_1100_1100_001);
    /// Interrupt Controller List Registers - 2
    pub const ICH_LR2_EL2: Self = Self::System(0b11_100_1100_1100_010);
    /// Interrupt Controller List Registers - 3
    pub const ICH_LR3_EL2: Self = Self::System(0b11_100_1100_1100_011);
    /// Interrupt Controller List Registers - 4
    pub const ICH_LR4_EL2: Self = Self::System(0b11_100_1100_1100_100);
    /// Interrupt Controller List Registers - 5
    pub const ICH_LR5_EL2: Self = Self::System(0b11_100_1100_1100_101);
    /// Interrupt Controller List Registers - 6
    pub const ICH_LR6_EL2: Self = Self::System(0b11_100_1100_1100_110);
    /// Interrupt Controller List Registers - 7
    pub const ICH_LR7_EL2: Self = Self::System(0b11_100_1100_1100_111);
    /// Interrupt Controller List Registers - 8
    pub const ICH_LR8_EL2: Self = Self::System(0b11_100_1100_1101_000);
    /// Interrupt Controller List Registers - 9
    pub const ICH_LR9_EL2: Self = Self::System(0b11_100_1100_1101_001);
    /// Interrupt Controller List Registers - 10
    pub const ICH_LR10_EL2: Self = Self::System(0b11_100_1100_1101_010);
    /// Interrupt Controller List Registers - 11
    pub const ICH_LR11_EL2: Self = Self::System(0b11_100_1100_1101_011);
    /// Interrupt Controller List Registers - 12
    pub const ICH_LR12_EL2: Self = Self::System(0b11_100_1100_1101_100);
    /// Interrupt Controller List Registers - 13
    pub const ICH_LR13_EL2: Self = Self::System(0b11_100_1100_1101_101);
    /// Interrupt Controller List Registers - 14
    pub const ICH_LR14_EL2: Self = Self::System(0b11_100_1100_1101_110);
    /// Interrupt Controller List Registers - 15
    pub const ICH_LR15_EL2: Self = Self::System(0b11_100_1100_1101_111);
    /// Context ID Register (EL2)
    pub const CONTEXTIDR_EL2: Self = Self::System(0b11_100_1101_0000_001);
    /// EL2 Software Thread ID Register
    pub const TPIDR_EL2: Self = Self::System(0b11_100_1101_0000_010);
    /// EL2 Read/Write Software Context Number
    pub const SCXTNUM_EL2: Self = Self::System(0b11_100_1101_0000_111);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 0
    pub const AMEVCNTVOFF00_EL2: Self = Self::System(0b11_100_1101_1000_000);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 1
    pub const AMEVCNTVOFF01_EL2: Self = Self::System(0b11_100_1101_1000_001);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 2
    pub const AMEVCNTVOFF02_EL2: Self = Self::System(0b11_100_1101_1000_010);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 3
    pub const AMEVCNTVOFF03_EL2: Self = Self::System(0b11_100_1101_1000_011);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 4
    pub const AMEVCNTVOFF04_EL2: Self = Self::System(0b11_100_1101_1000_100);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 5
    pub const AMEVCNTVOFF05_EL2: Self = Self::System(0b11_100_1101_1000_101);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 6
    pub const AMEVCNTVOFF06_EL2: Self = Self::System(0b11_100_1101_1000_110);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 7
    pub const AMEVCNTVOFF07_EL2: Self = Self::System(0b11_100_1101_1000_111);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 8
    pub const AMEVCNTVOFF08_EL2: Self = Self::System(0b11_100_1101_1001_000);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 9
    pub const AMEVCNTVOFF09_EL2: Self = Self::System(0b11_100_1101_1001_001);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 10
    pub const AMEVCNTVOFF010_EL2: Self = Self::System(0b11_100_1101_1001_010);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 11
    pub const AMEVCNTVOFF011_EL2: Self = Self::System(0b11_100_1101_1001_011);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 12
    pub const AMEVCNTVOFF012_EL2: Self = Self::System(0b11_100_1101_1001_100);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 13
    pub const AMEVCNTVOFF013_EL2: Self = Self::System(0b11_100_1101_1001_101);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 14
    pub const AMEVCNTVOFF014_EL2: Self = Self::System(0b11_100_1101_1001_110);
    /// Activity Monitors Event Counter Virtual Offset Registers 0 - 15
    pub const AMEVCNTVOFF015_EL2: Self = Self::System(0b11_100_1101_1001_111);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 0
    pub const AMEVCNTVOFF10_EL2: Self = Self::System(0b11_100_1101_1010_000);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 1
    pub const AMEVCNTVOFF11_EL2: Self = Self::System(0b11_100_1101_1010_001);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 2
    pub const AMEVCNTVOFF12_EL2: Self = Self::System(0b11_100_1101_1010_010);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 3
    pub const AMEVCNTVOFF13_EL2: Self = Self::System(0b11_100_1101_1010_011);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 4
    pub const AMEVCNTVOFF14_EL2: Self = Self::System(0b11_100_1101_1010_100);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 5
    pub const AMEVCNTVOFF15_EL2: Self = Self::System(0b11_100_1101_1010_101);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 6
    pub const AMEVCNTVOFF16_EL2: Self = Self::System(0b11_100_1101_1010_110);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 7
    pub const AMEVCNTVOFF17_EL2: Self = Self::System(0b11_100_1101_1010_111);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 8
    pub const AMEVCNTVOFF18_EL2: Self = Self::System(0b11_100_1101_1011_000);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 9
    pub const AMEVCNTVOFF19_EL2: Self = Self::System(0b11_100_1101_1011_001);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 10
    pub const AMEVCNTVOFF110_EL2: Self = Self::System(0b11_100_1101_1011_010);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 11
    pub const AMEVCNTVOFF111_EL2: Self = Self::System(0b11_100_1101_1011_011);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 12
    pub const AMEVCNTVOFF112_EL2: Self = Self::System(0b11_100_1101_1011_100);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 13
    pub const AMEVCNTVOFF113_EL2: Self = Self::System(0b11_100_1101_1011_101);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 14
    pub const AMEVCNTVOFF114_EL2: Self = Self::System(0b11_100_1101_1011_110);
    /// Activity Monitors Event Counter Virtual Offset Registers 1 - 15
    pub const AMEVCNTVOFF115_EL2: Self = Self::System(0b11_100_1101_1011_111);
    /// Counter-timer Virtual Offset Register
    pub const CNTVOFF_EL2: Self = Self::System(0b11_100_1110_0000_011);
    /// Counter-timer Physical Offset Register
    pub const CNTPOFF_EL2: Self = Self::System(0b11_100_1110_0000_110);
    /// Counter-timer Hypervisor Control Register
    pub const CNTHCTL_EL2: Self = Self::System(0b11_100_1110_0001_000);
    /// Counter-timer Physical Timer TimerValue Register (EL2)
    pub const CNTHP_TVAL_EL2: Self = Self::System(0b11_100_1110_0010_000);
    /// Counter-timer Hypervisor Physical Timer Control Register
    pub const CNTHP_CTL_EL2: Self = Self::System(0b11_100_1110_0010_001);
    /// Counter-timer Physical Timer CompareValue Register (EL2)
    pub const CNTHP_CVAL_EL2: Self = Self::System(0b11_100_1110_0010_010);
    /// Counter-timer Virtual Timer TimerValue Register (EL2)
    pub const CNTHV_TVAL_EL2: Self = Self::System(0b11_100_1110_0011_000);
    /// Counter-timer Virtual Timer Control Register (EL2)
    pub const CNTHV_CTL_EL2: Self = Self::System(0b11_100_1110_0011_001);
    /// Counter-timer Virtual Timer CompareValue Register (EL2)
    pub const CNTHV_CVAL_EL2: Self = Self::System(0b11_100_1110_0011_010);
    /// Counter-timer Secure Virtual Timer TimerValue Register (EL2)
    pub const CNTHVS_TVAL_EL2: Self = Self::System(0b11_100_1110_0100_000);
    /// Counter-timer Secure Virtual Timer Control Register (EL2)
    pub const CNTHVS_CTL_EL2: Self = Self::System(0b11_100_1110_0100_001);
    /// Counter-timer Secure Virtual Timer CompareValue Register (EL2)
    pub const CNTHVS_CVAL_EL2: Self = Self::System(0b11_100_1110_0100_010);
    /// Counter-timer Secure Physical Timer TimerValue Register (EL2)
    pub const CNTHPS_TVAL_EL2: Self = Self::System(0b11_100_1110_0101_000);
    /// Counter-timer Secure Physical Timer Control Register (EL2)
    pub const CNTHPS_CTL_EL2: Self = Self::System(0b11_100_1110_0101_001);
    /// Counter-timer Secure Physical Timer CompareValue Register (EL2)
    pub const CNTHPS_CVAL_EL2: Self = Self::System(0b11_100_1110_0101_010);
    /// System Control Register (EL3)
    pub const SCTLR_EL3: Self = Self::System(0b11_110_0001_0000_000);
    /// Auxiliary Control Register (EL3)
    pub const ACTLR_EL3: Self = Self::System(0b11_110_0001_0000_001);
    /// Secure Configuration Register
    pub const SCR_EL3: Self = Self::System(0b11_110_0001_0001_000);
    /// AArch32 Secure Debug Enable Register
    pub const SDER32_EL3: Self = Self::System(0b11_110_0001_0001_001);
    /// Architectural Feature Trap Register (EL3)
    pub const CPTR_EL3: Self = Self::System(0b11_110_0001_0001_010);
    /// SVE Control Register (EL3)
    pub const ZCR_EL3: Self = Self::System(0b11_110_0001_0010_000);
    /// SME Control Register (EL3)
    pub const SMCR_EL3: Self = Self::System(0b11_110_0001_0010_110);
    /// Monitor Debug Configuration Register (EL3)
    pub const MDCR_EL3: Self = Self::System(0b11_110_0001_0011_001);
    /// Translation Table Base Register 0 (EL3)
    pub const TTBR0_EL3: Self = Self::System(0b11_110_0010_0000_000);
    /// Translation Control Register (EL3)
    pub const TCR_EL3: Self = Self::System(0b11_110_0010_0000_010);
    /// Granule Protection Table Base Register
    pub const GPTBR_EL3: Self = Self::System(0b11_110_0010_0001_100);
    /// Granule Protection Check Control Register (EL3)
    pub const GPCCR_EL3: Self = Self::System(0b11_110_0010_0001_110);
    /// Saved Program Status Register (EL3)
    pub const SPSR_EL3: Self = Self::System(0b11_110_0100_0000_000);
    /// Exception Link Register (EL3)
    pub const ELR_EL3: Self = Self::System(0b11_110_0100_0000_001);
    /// Stack Pointer (EL2)
    pub const SP_EL2: Self = Self::System(0b11_110_0100_0001_000);
    /// Auxiliary Fault Status Register 0 (EL3)
    pub const AFSR0_EL3: Self = Self::System(0b11_110_0101_0001_000);
    /// Auxiliary Fault Status Register 1 (EL3)
    pub const AFSR1_EL3: Self = Self::System(0b11_110_0101_0001_001);
    /// Exception Syndrome Register (EL3)
    pub const ESR_EL3: Self = Self::System(0b11_110_0101_0010_000);
    /// Tag Fault Status Register (EL3)
    pub const TFSR_EL3: Self = Self::System(0b11_110_0101_0110_000);
    /// Fault Address Register (EL3)
    pub const FAR_EL3: Self = Self::System(0b11_110_0110_0000_000);
    /// PA Fault Address Register
    pub const MFAR_EL3: Self = Self::System(0b11_110_0110_0000_101);
    /// Memory Attribute Indirection Register (EL3)
    pub const MAIR_EL3: Self = Self::System(0b11_110_1010_0010_000);
    /// Auxiliary Memory Attribute Indirection Register (EL3)
    pub const AMAIR_EL3: Self = Self::System(0b11_110_1010_0011_000);
    /// MPAM3 Register (EL3)
    pub const MPAM3_EL3: Self = Self::System(0b11_110_1010_0101_000);
    /// Vector Base Address Register (EL3)
    pub const VBAR_EL3: Self = Self::System(0b11_110_1100_0000_000);
    /// Reset Vector Base Address Register (if EL3 Implemented)
    pub const RVBAR_EL3: Self = Self::System(0b11_110_1100_0000_001);
    /// Reset Management Register (EL3)
    pub const RMR_EL3: Self = Self::System(0b11_110_1100_0000_010);
    /// Interrupt Controller Control Register (EL3)
    pub const ICC_CTLR_EL3: Self = Self::System(0b11_110_1100_1100_100);
    /// Interrupt Controller System Register Enable Register (EL3)
    pub const ICC_SRE_EL3: Self = Self::System(0b11_110_1100_1100_101);
    /// Interrupt Controller Interrupt Group 1 Enable Register (EL3)
    pub const ICC_IGRPEN1_EL3: Self = Self::System(0b11_110_1100_1100_111);
    /// EL3 Software Thread ID Register
    pub const TPIDR_EL3: Self = Self::System(0b11_110_1101_0000_010);
    /// EL3 Read/Write Software Context Number
    pub const SCXTNUM_EL3: Self = Self::System(0b11_110_1101_0000_111);
    /// Counter-timer Physical Secure Timer TimerValue Register
    pub const CNTPS_TVAL_EL1: Self = Self::System(0b11_111_1110_0010_000);
    /// Counter-timer Physical Secure Timer Control Register
    pub const CNTPS_CTL_EL1: Self = Self::System(0b11_111_1110_0010_001);
    /// Counter-timer Physical Secure Timer CompareValue Register
    pub const CNTPS_CVAL_EL1: Self = Self::System(0b11_111_1110_0010_010);

    /// OS Lock Data Transfer Register, Receive
    pub const OSDTRRX_EL1: Self = Self::System(0b10_000_0000_0000_010);
    /// Debug Breakpoint Value Registers - 0
    pub const DBGBVR0_EL1: Self = Self::System(0b10_000_0000_0000_100);
    /// Debug Breakpoint Control Registers - 0
    pub const DBGBCR0_EL1: Self = Self::System(0b10_000_0000_0000_101);
    /// Debug Watchpoint Value Registers - 0
    pub const DBGWVR0_EL1: Self = Self::System(0b10_000_0000_0000_110);
    /// Debug Watchpoint Control Registers - 0
    pub const DBGWCR0_EL1: Self = Self::System(0b10_000_0000_0000_111);
    /// Debug Breakpoint Value Registers - 1
    pub const DBGBVR1_EL1: Self = Self::System(0b10_000_0000_0001_100);
    /// Debug Breakpoint Control Registers - 1
    pub const DBGBCR1_EL1: Self = Self::System(0b10_000_0000_0001_101);
    /// Debug Watchpoint Value Registers - 1
    pub const DBGWVR1_EL1: Self = Self::System(0b10_000_0000_0001_110);
    /// Debug Watchpoint Control Registers - 1
    pub const DBGWCR1_EL1: Self = Self::System(0b10_000_0000_0001_111);
    /// Monitor DCC Interrupt Enable Register
    pub const MDCCINT_EL1: Self = Self::System(0b10_000_0000_0010_000);
    /// Monitor Debug System Control Register
    pub const MDSCR_EL1: Self = Self::System(0b10_000_0000_0010_010);
    /// Debug Breakpoint Value Registers - 2
    pub const DBGBVR2_EL1: Self = Self::System(0b10_000_0000_0010_100);
    /// Debug Breakpoint Control Registers - 2
    pub const DBGBCR2_EL1: Self = Self::System(0b10_000_0000_0010_101);
    /// Debug Watchpoint Value Registers - 2
    pub const DBGWVR2_EL1: Self = Self::System(0b10_000_0000_0010_110);
    /// Debug Watchpoint Control Registers - 2
    pub const DBGWCR2_EL1: Self = Self::System(0b10_000_0000_0010_111);
    /// OS Lock Data Transfer Register, Transmit
    pub const OSDTRTX_EL1: Self = Self::System(0b10_000_0000_0011_010);
    /// Debug Breakpoint Value Registers - 3
    pub const DBGBVR3_EL1: Self = Self::System(0b10_000_0000_0011_100);
    /// Debug Breakpoint Control Registers - 3
    pub const DBGBCR3_EL1: Self = Self::System(0b10_000_0000_0011_101);
    /// Debug Watchpoint Value Registers - 3
    pub const DBGWVR3_EL1: Self = Self::System(0b10_000_0000_0011_110);
    /// Debug Watchpoint Control Registers - 3
    pub const DBGWCR3_EL1: Self = Self::System(0b10_000_0000_0011_111);
    /// Debug Breakpoint Value Registers - 4
    pub const DBGBVR4_EL1: Self = Self::System(0b10_000_0000_0100_100);
    /// Debug Breakpoint Control Registers - 4
    pub const DBGBCR4_EL1: Self = Self::System(0b10_000_0000_0100_101);
    /// Debug Watchpoint Value Registers - 4
    pub const DBGWVR4_EL1: Self = Self::System(0b10_000_0000_0100_110);
    /// Debug Watchpoint Control Registers - 4
    pub const DBGWCR4_EL1: Self = Self::System(0b10_000_0000_0100_111);
    /// Debug Breakpoint Value Registers - 5
    pub const DBGBVR5_EL1: Self = Self::System(0b10_000_0000_0101_100);
    /// Debug Breakpoint Control Registers - 5
    pub const DBGBCR5_EL1: Self = Self::System(0b10_000_0000_0101_101);
    /// Debug Watchpoint Value Registers - 5
    pub const DBGWVR5_EL1: Self = Self::System(0b10_000_0000_0101_110);
    /// Debug Watchpoint Control Registers - 5
    pub const DBGWCR5_EL1: Self = Self::System(0b10_000_0000_0101_111);
    /// OS Lock Exception Catch Control Register
    pub const OSECCR_EL1: Self = Self::System(0b10_000_0000_0110_010);
    /// Debug Breakpoint Value Registers - 6
    pub const DBGBVR6_EL1: Self = Self::System(0b10_000_0000_0110_100);
    /// Debug Breakpoint Control Registers - 6
    pub const DBGBCR6_EL1: Self = Self::System(0b10_000_0000_0110_101);
    /// Debug Watchpoint Value Registers - 6
    pub const DBGWVR6_EL1: Self = Self::System(0b10_000_0000_0110_110);
    /// Debug Watchpoint Control Registers - 6
    pub const DBGWCR6_EL1: Self = Self::System(0b10_000_0000_0110_111);
    /// Debug Breakpoint Value Registers - 7
    pub const DBGBVR7_EL1: Self = Self::System(0b10_000_0000_0111_100);
    /// Debug Breakpoint Control Registers - 7
    pub const DBGBCR7_EL1: Self = Self::System(0b10_000_0000_0111_101);
    /// Debug Watchpoint Value Registers - 7
    pub const DBGWVR7_EL1: Self = Self::System(0b10_000_0000_0111_110);
    /// Debug Watchpoint Control Registers - 7
    pub const DBGWCR7_EL1: Self = Self::System(0b10_000_0000_0111_111);
    /// Debug Breakpoint Value Registers - 8
    pub const DBGBVR8_EL1: Self = Self::System(0b10_000_0000_1000_100);
    /// Debug Breakpoint Control Registers - 8
    pub const DBGBCR8_EL1: Self = Self::System(0b10_000_0000_1000_101);
    /// Debug Watchpoint Value Registers - 8
    pub const DBGWVR8_EL1: Self = Self::System(0b10_000_0000_1000_110);
    /// Debug Watchpoint Control Registers - 8
    pub const DBGWCR8_EL1: Self = Self::System(0b10_000_0000_1000_111);
    /// Debug Breakpoint Value Registers - 9
    pub const DBGBVR9_EL1: Self = Self::System(0b10_000_0000_1001_100);
    /// Debug Breakpoint Control Registers - 9
    pub const DBGBCR9_EL1: Self = Self::System(0b10_000_0000_1001_101);
    /// Debug Watchpoint Value Registers - 9
    pub const DBGWVR9_EL1: Self = Self::System(0b10_000_0000_1001_110);
    /// Debug Watchpoint Control Registers - 9
    pub const DBGWCR9_EL1: Self = Self::System(0b10_000_0000_1001_111);
    /// Debug Breakpoint Value Registers - 10
    pub const DBGBVR10_EL1: Self = Self::System(0b10_000_0000_1010_100);
    /// Debug Breakpoint Control Registers - 10
    pub const DBGBCR10_EL1: Self = Self::System(0b10_000_0000_1010_101);
    /// Debug Watchpoint Value Registers - 10
    pub const DBGWVR10_EL1: Self = Self::System(0b10_000_0000_1010_110);
    /// Debug Watchpoint Control Registers - 10
    pub const DBGWCR10_EL1: Self = Self::System(0b10_000_0000_1010_111);
    /// Debug Breakpoint Value Registers - 11
    pub const DBGBVR11_EL1: Self = Self::System(0b10_000_0000_1011_100);
    /// Debug Breakpoint Control Registers - 11
    pub const DBGBCR11_EL1: Self = Self::System(0b10_000_0000_1011_101);
    /// Debug Watchpoint Value Registers - 11
    pub const DBGWVR11_EL1: Self = Self::System(0b10_000_0000_1011_110);
    /// Debug Watchpoint Control Registers - 11
    pub const DBGWCR11_EL1: Self = Self::System(0b10_000_0000_1011_111);
    /// Debug Breakpoint Value Registers - 12
    pub const DBGBVR12_EL1: Self = Self::System(0b10_000_0000_1100_100);
    /// Debug Breakpoint Control Registers - 12
    pub const DBGBCR12_EL1: Self = Self::System(0b10_000_0000_1100_101);
    /// Debug Watchpoint Value Registers - 12
    pub const DBGWVR12_EL1: Self = Self::System(0b10_000_0000_1100_110);
    /// Debug Watchpoint Control Registers - 12
    pub const DBGWCR12_EL1: Self = Self::System(0b10_000_0000_1100_111);
    /// Debug Breakpoint Value Registers - 13
    pub const DBGBVR13_EL1: Self = Self::System(0b10_000_0000_1101_100);
    /// Debug Breakpoint Control Registers - 13
    pub const DBGBCR13_EL1: Self = Self::System(0b10_000_0000_1101_101);
    /// Debug Watchpoint Value Registers - 13
    pub const DBGWVR13_EL1: Self = Self::System(0b10_000_0000_1101_110);
    /// Debug Watchpoint Control Registers - 13
    pub const DBGWCR13_EL1: Self = Self::System(0b10_000_0000_1101_111);
    /// Debug Breakpoint Value Registers - 14
    pub const DBGBVR14_EL1: Self = Self::System(0b10_000_0000_1110_100);
    /// Debug Breakpoint Control Registers - 14
    pub const DBGBCR14_EL1: Self = Self::System(0b10_000_0000_1110_101);
    /// Debug Watchpoint Value Registers - 14
    pub const DBGWVR14_EL1: Self = Self::System(0b10_000_0000_1110_110);
    /// Debug Watchpoint Control Registers - 14
    pub const DBGWCR14_EL1: Self = Self::System(0b10_000_0000_1110_111);
    /// Debug Breakpoint Value Registers - 15
    pub const DBGBVR15_EL1: Self = Self::System(0b10_000_0000_1111_100);
    /// Debug Breakpoint Control Registers - 15
    pub const DBGBCR15_EL1: Self = Self::System(0b10_000_0000_1111_101);
    /// Debug Watchpoint Value Registers - 15
    pub const DBGWVR15_EL1: Self = Self::System(0b10_000_0000_1111_110);
    /// Debug Watchpoint Control Registers - 15
    pub const DBGWCR15_EL1: Self = Self::System(0b10_000_0000_1111_111);
    /// Monitor Debug ROM Address Register
    pub const MDRAR_EL1: Self = Self::System(0b10_000_0001_0000_000);
    /// OS Lock Access Register
    pub const OSLAR_EL1: Self = Self::System(0b10_000_0001_0000_100);
    /// OS Lock Status Register
    pub const OSLSR_EL1: Self = Self::System(0b10_000_0001_0001_100);
    /// OS Double Lock Register
    pub const OSDLR_EL1: Self = Self::System(0b10_000_0001_0011_100);
    /// Debug Power Control Register
    pub const DBGPRCR_EL1: Self = Self::System(0b10_000_0001_0100_100);
    /// Debug CLAIM Tag Set Register
    pub const DBGCLAIMSET_EL1: Self = Self::System(0b10_000_0111_1000_110);
    /// Debug CLAIM Tag Clear Register
    pub const DBGCLAIMCLR_EL1: Self = Self::System(0b10_000_0111_1001_110);
    /// Debug Authentication Status Register
    pub const DBGAUTHSTATUS_EL1: Self = Self::System(0b10_000_0111_1110_110);
    /// Trace ID Register
    pub const TRCTRACEIDR: Self = Self::System(0b10_001_0000_0000_001);
    /// ViewInst Main Control Register
    pub const TRCVICTLR: Self = Self::System(0b10_001_0000_0000_010);
    /// Sequencer State Transition Control Register 0
    pub const TRCSEQEVR0: Self = Self::System(0b10_001_0000_0000_100);
    /// Counter Reload Value Register 0
    pub const TRCCNTRLDVR0: Self = Self::System(0b10_001_0000_0000_101);
    /// ID Register 8
    pub const TRCIDR8: Self = Self::System(0b10_001_0000_0000_110);
    /// IMP DEF Register 0
    pub const TRCIMSPEC0: Self = Self::System(0b10_001_0000_0000_111);
    /// Programming Control Register
    pub const TRCPRGCTLR: Self = Self::System(0b10_001_0000_0001_000);
    /// Q Element Control Register
    pub const TRCQCTLR: Self = Self::System(0b10_001_0000_0001_001);
    /// ViewInst Include/Exclude Control Register
    pub const TRCVIIECTLR: Self = Self::System(0b10_001_0000_0001_010);
    /// Sequencer State Transition Control Register 1
    pub const TRCSEQEVR1: Self = Self::System(0b10_001_0000_0001_100);
    /// Counter Reload Value Register 1
    pub const TRCCNTRLDVR1: Self = Self::System(0b10_001_0000_0001_101);
    /// ID Register 9
    pub const TRCIDR9: Self = Self::System(0b10_001_0000_0001_110);
    /// IMP DEF Register 1
    pub const TRCIMSPEC1: Self = Self::System(0b10_001_0000_0001_111);
    /// ViewInst Start/Stop Control Register
    pub const TRCVISSCTLR: Self = Self::System(0b10_001_0000_0010_010);
    /// Sequencer State Transition Control Register 2
    pub const TRCSEQEVR2: Self = Self::System(0b10_001_0000_0010_100);
    /// Counter Reload Value Register 2
    pub const TRCCNTRLDVR2: Self = Self::System(0b10_001_0000_0010_101);
    /// ID Register 10
    pub const TRCIDR10: Self = Self::System(0b10_001_0000_0010_110);
    /// IMP DEF Register 2
    pub const TRCIMSPEC2: Self = Self::System(0b10_001_0000_0010_111);
    /// Trace Status Register
    pub const TRCSTATR: Self = Self::System(0b10_001_0000_0011_000);
    /// ViewInst Start/Stop PE Comparator Control Register
    pub const TRCVIPCSSCTLR: Self = Self::System(0b10_001_0000_0011_010);
    /// Counter Reload Value Register 3
    pub const TRCCNTRLDVR3: Self = Self::System(0b10_001_0000_0011_101);
    /// ID Register 11
    pub const TRCIDR11: Self = Self::System(0b10_001_0000_0011_110);
    /// IMP DEF Register 3
    pub const TRCIMSPEC3: Self = Self::System(0b10_001_0000_0011_111);
    /// Trace Configuration Register
    pub const TRCCONFIGR: Self = Self::System(0b10_001_0000_0100_000);
    /// Counter Control Register 0
    pub const TRCCNTCTLR0: Self = Self::System(0b10_001_0000_0100_101);
    /// ID Register 12
    pub const TRCIDR12: Self = Self::System(0b10_001_0000_0100_110);
    /// IMP DEF Register 4
    pub const TRCIMSPEC4: Self = Self::System(0b10_001_0000_0100_111);
    /// Counter Control Register 1
    pub const TRCCNTCTLR1: Self = Self::System(0b10_001_0000_0101_101);
    /// ID Register 13
    pub const TRCIDR13: Self = Self::System(0b10_001_0000_0101_110);
    /// IMP DEF Register 5
    pub const TRCIMSPEC5: Self = Self::System(0b10_001_0000_0101_111);
    /// Auxiliary Control Register
    pub const TRCAUXCTLR: Self = Self::System(0b10_001_0000_0110_000);
    /// Sequencer Reset Control Register
    pub const TRCSEQRSTEVR: Self = Self::System(0b10_001_0000_0110_100);
    /// Counter Control Register 2
    pub const TRCCNTCTLR2: Self = Self::System(0b10_001_0000_0110_101);
    /// IMP DEF Register 6
    pub const TRCIMSPEC6: Self = Self::System(0b10_001_0000_0110_111);
    /// Sequencer State Register
    pub const TRCSEQSTR: Self = Self::System(0b10_001_0000_0111_100);
    /// Counter Control Register 3
    pub const TRCCNTCTLR3: Self = Self::System(0b10_001_0000_0111_101);
    /// IMP DEF Register 7
    pub const TRCIMSPEC7: Self = Self::System(0b10_001_0000_0111_111);
    /// Event Control 0 Register
    pub const TRCEVENTCTL0R: Self = Self::System(0b10_001_0000_1000_000);
    /// External Input Select Register 0
    pub const TRCEXTINSELR0: Self = Self::System(0b10_001_0000_1000_100);
    /// Counter Value Register 0
    pub const TRCCNTVR0: Self = Self::System(0b10_001_0000_1000_101);
    /// ID Register 0
    pub const TRCIDR0: Self = Self::System(0b10_001_0000_1000_111);
    /// Event Control 1 Register
    pub const TRCEVENTCTL1R: Self = Self::System(0b10_001_0000_1001_000);
    /// External Input Select Register 1
    pub const TRCEXTINSELR1: Self = Self::System(0b10_001_0000_1001_100);
    /// Counter Value Register 1
    pub const TRCCNTVR1: Self = Self::System(0b10_001_0000_1001_101);
    /// ID Register 1
    pub const TRCIDR1: Self = Self::System(0b10_001_0000_1001_111);
    /// Resources Status Register
    pub const TRCRSR: Self = Self::System(0b10_001_0000_1010_000);
    /// External Input Select Register 2
    pub const TRCEXTINSELR2: Self = Self::System(0b10_001_0000_1010_100);
    /// Counter Value Register 2
    pub const TRCCNTVR2: Self = Self::System(0b10_001_0000_1010_101);
    /// ID Register 2
    pub const TRCIDR2: Self = Self::System(0b10_001_0000_1010_111);
    /// Stall Control Register
    pub const TRCSTALLCTLR: Self = Self::System(0b10_001_0000_1011_000);
    /// External Input Select Register 3
    pub const TRCEXTINSELR3: Self = Self::System(0b10_001_0000_1011_100);
    /// Counter Value Register 3
    pub const TRCCNTVR3: Self = Self::System(0b10_001_0000_1011_101);
    /// ID Register 3
    pub const TRCIDR3: Self = Self::System(0b10_001_0000_1011_111);
    /// Timestamp Control Register
    pub const TRCTSCTLR: Self = Self::System(0b10_001_0000_1100_000);
    /// ID Register 4
    pub const TRCIDR4: Self = Self::System(0b10_001_0000_1100_111);
    /// Synchronization Period Register
    pub const TRCSYNCPR: Self = Self::System(0b10_001_0000_1101_000);
    /// ID Register 5
    pub const TRCIDR5: Self = Self::System(0b10_001_0000_1101_111);
    /// Cycle Count Control Register
    pub const TRCCCCTLR: Self = Self::System(0b10_001_0000_1110_000);
    /// ID Register 6
    pub const TRCIDR6: Self = Self::System(0b10_001_0000_1110_111);
    /// Branch Broadcast Control Register
    pub const TRCBBCTLR: Self = Self::System(0b10_001_0000_1111_000);
    /// ID Register 7
    pub const TRCIDR7: Self = Self::System(0b10_001_0000_1111_111);
    /// Resource Selection Control Register 16
    pub const TRCRSCTLR16: Self = Self::System(0b10_001_0001_0000_001);
    /// Single-shot Comparator Control Register 0
    pub const TRCSSCCR0: Self = Self::System(0b10_001_0001_0000_010);
    /// Single-shot Processing Element Comparator Input Control Register 0
    pub const TRCSSPCICR0: Self = Self::System(0b10_001_0001_0000_011);
    /// Resource Selection Control Register 17
    pub const TRCRSCTLR17: Self = Self::System(0b10_001_0001_0001_001);
    /// Single-shot Comparator Control Register 1
    pub const TRCSSCCR1: Self = Self::System(0b10_001_0001_0001_010);
    /// Single-shot Processing Element Comparator Input Control Register 1
    pub const TRCSSPCICR1: Self = Self::System(0b10_001_0001_0001_011);
    /// Trace OS Lock Status Register
    pub const TRCOSLSR: Self = Self::System(0b10_001_0001_0001_100);
    /// Resource Selection Control Register 2
    pub const TRCRSCTLR2: Self = Self::System(0b10_001_0001_0010_000);
    /// Resource Selection Control Register 18
    pub const TRCRSCTLR18: Self = Self::System(0b10_001_0001_0010_001);
    /// Single-shot Comparator Control Register 2
    pub const TRCSSCCR2: Self = Self::System(0b10_001_0001_0010_010);
    /// Single-shot Processing Element Comparator Input Control Register 2
    pub const TRCSSPCICR2: Self = Self::System(0b10_001_0001_0010_011);
    /// Resource Selection Control Register 3
    pub const TRCRSCTLR3: Self = Self::System(0b10_001_0001_0011_000);
    /// Resource Selection Control Register 19
    pub const TRCRSCTLR19: Self = Self::System(0b10_001_0001_0011_001);
    /// Single-shot Comparator Control Register 3
    pub const TRCSSCCR3: Self = Self::System(0b10_001_0001_0011_010);
    /// Single-shot Processing Element Comparator Input Control Register 3
    pub const TRCSSPCICR3: Self = Self::System(0b10_001_0001_0011_011);
    /// Resource Selection Control Register 4
    pub const TRCRSCTLR4: Self = Self::System(0b10_001_0001_0100_000);
    /// Resource Selection Control Register 20
    pub const TRCRSCTLR20: Self = Self::System(0b10_001_0001_0100_001);
    /// Single-shot Comparator Control Register 4
    pub const TRCSSCCR4: Self = Self::System(0b10_001_0001_0100_010);
    /// Single-shot Processing Element Comparator Input Control Register 4
    pub const TRCSSPCICR4: Self = Self::System(0b10_001_0001_0100_011);
    /// Resource Selection Control Register 5
    pub const TRCRSCTLR5: Self = Self::System(0b10_001_0001_0101_000);
    /// Resource Selection Control Register 21
    pub const TRCRSCTLR21: Self = Self::System(0b10_001_0001_0101_001);
    /// Single-shot Comparator Control Register 5
    pub const TRCSSCCR5: Self = Self::System(0b10_001_0001_0101_010);
    /// Single-shot Processing Element Comparator Input Control Register 5
    pub const TRCSSPCICR5: Self = Self::System(0b10_001_0001_0101_011);
    /// Resource Selection Control Register 6
    pub const TRCRSCTLR6: Self = Self::System(0b10_001_0001_0110_000);
    /// Resource Selection Control Register 22
    pub const TRCRSCTLR22: Self = Self::System(0b10_001_0001_0110_001);
    /// Single-shot Comparator Control Register 6
    pub const TRCSSCCR6: Self = Self::System(0b10_001_0001_0110_010);
    /// Single-shot Processing Element Comparator Input Control Register 6
    pub const TRCSSPCICR6: Self = Self::System(0b10_001_0001_0110_011);
    /// Resource Selection Control Register 7
    pub const TRCRSCTLR7: Self = Self::System(0b10_001_0001_0111_000);
    /// Resource Selection Control Register 23
    pub const TRCRSCTLR23: Self = Self::System(0b10_001_0001_0111_001);
    /// Single-shot Comparator Control Register 7
    pub const TRCSSCCR7: Self = Self::System(0b10_001_0001_0111_010);
    /// Single-shot Processing Element Comparator Input Control Register 7
    pub const TRCSSPCICR7: Self = Self::System(0b10_001_0001_0111_011);
    /// Resource Selection Control Register 8
    pub const TRCRSCTLR8: Self = Self::System(0b10_001_0001_1000_000);
    /// Resource Selection Control Register 24
    pub const TRCRSCTLR24: Self = Self::System(0b10_001_0001_1000_001);
    /// Single-shot Comparator Control Status Register 0
    pub const TRCSSCSR0: Self = Self::System(0b10_001_0001_1000_010);
    /// Resource Selection Control Register 9
    pub const TRCRSCTLR9: Self = Self::System(0b10_001_0001_1001_000);
    /// Resource Selection Control Register 25
    pub const TRCRSCTLR25: Self = Self::System(0b10_001_0001_1001_001);
    /// Single-shot Comparator Control Status Register 1
    pub const TRCSSCSR1: Self = Self::System(0b10_001_0001_1001_010);
    /// Resource Selection Control Register 10
    pub const TRCRSCTLR10: Self = Self::System(0b10_001_0001_1010_000);
    /// Resource Selection Control Register 26
    pub const TRCRSCTLR26: Self = Self::System(0b10_001_0001_1010_001);
    /// Single-shot Comparator Control Status Register 2
    pub const TRCSSCSR2: Self = Self::System(0b10_001_0001_1010_010);
    /// Resource Selection Control Register 11
    pub const TRCRSCTLR11: Self = Self::System(0b10_001_0001_1011_000);
    /// Resource Selection Control Register 27
    pub const TRCRSCTLR27: Self = Self::System(0b10_001_0001_1011_001);
    /// Single-shot Comparator Control Status Register 3
    pub const TRCSSCSR3: Self = Self::System(0b10_001_0001_1011_010);
    /// Resource Selection Control Register 12
    pub const TRCRSCTLR12: Self = Self::System(0b10_001_0001_1100_000);
    /// Resource Selection Control Register 28
    pub const TRCRSCTLR28: Self = Self::System(0b10_001_0001_1100_001);
    /// Single-shot Comparator Control Status Register 4
    pub const TRCSSCSR4: Self = Self::System(0b10_001_0001_1100_010);
    /// Resource Selection Control Register 13
    pub const TRCRSCTLR13: Self = Self::System(0b10_001_0001_1101_000);
    /// Resource Selection Control Register 29
    pub const TRCRSCTLR29: Self = Self::System(0b10_001_0001_1101_001);
    /// Single-shot Comparator Control Status Register 5
    pub const TRCSSCSR5: Self = Self::System(0b10_001_0001_1101_010);
    /// Resource Selection Control Register 14
    pub const TRCRSCTLR14: Self = Self::System(0b10_001_0001_1110_000);
    /// Resource Selection Control Register 30
    pub const TRCRSCTLR30: Self = Self::System(0b10_001_0001_1110_001);
    /// Single-shot Comparator Control Status Register 6
    pub const TRCSSCSR6: Self = Self::System(0b10_001_0001_1110_010);
    /// Resource Selection Control Register 15
    pub const TRCRSCTLR15: Self = Self::System(0b10_001_0001_1111_000);
    /// Resource Selection Control Register 31
    pub const TRCRSCTLR31: Self = Self::System(0b10_001_0001_1111_001);
    /// Single-shot Comparator Control Status Register 7
    pub const TRCSSCSR7: Self = Self::System(0b10_001_0001_1111_010);
    /// Address Comparator Value Register 0
    pub const TRCACVR0: Self = Self::System(0b10_001_0010_0000_000);
    /// Address Comparator Value Register 8
    pub const TRCACVR8: Self = Self::System(0b10_001_0010_0000_001);
    /// Address Comparator Access Type Register 0
    pub const TRCACATR0: Self = Self::System(0b10_001_0010_0000_010);
    /// Address Comparator Access Type Register 8
    pub const TRCACATR8: Self = Self::System(0b10_001_0010_0000_011);
    /// Address Comparator Value Register 1
    pub const TRCACVR1: Self = Self::System(0b10_001_0010_0010_000);
    /// Address Comparator Value Register 9
    pub const TRCACVR9: Self = Self::System(0b10_001_0010_0010_001);
    /// Address Comparator Access Type Register 1
    pub const TRCACATR1: Self = Self::System(0b10_001_0010_0010_010);
    /// Address Comparator Access Type Register 9
    pub const TRCACATR9: Self = Self::System(0b10_001_0010_0010_011);
    /// Address Comparator Value Register 2
    pub const TRCACVR2: Self = Self::System(0b10_001_0010_0100_000);
    /// Address Comparator Value Register 10
    pub const TRCACVR10: Self = Self::System(0b10_001_0010_0100_001);
    /// Address Comparator Access Type Register 2
    pub const TRCACATR2: Self = Self::System(0b10_001_0010_0100_010);
    /// Address Comparator Access Type Register 10
    pub const TRCACATR10: Self = Self::System(0b10_001_0010_0100_011);
    /// Address Comparator Value Register 3
    pub const TRCACVR3: Self = Self::System(0b10_001_0010_0110_000);
    /// Address Comparator Value Register 11
    pub const TRCACVR11: Self = Self::System(0b10_001_0010_0110_001);
    /// Address Comparator Access Type Register 3
    pub const TRCACATR3: Self = Self::System(0b10_001_0010_0110_010);
    /// Address Comparator Access Type Register 11
    pub const TRCACATR11: Self = Self::System(0b10_001_0010_0110_011);
    /// Address Comparator Value Register 4
    pub const TRCACVR4: Self = Self::System(0b10_001_0010_1000_000);
    /// Address Comparator Value Register 12
    pub const TRCACVR12: Self = Self::System(0b10_001_0010_1000_001);
    /// Address Comparator Access Type Register 4
    pub const TRCACATR4: Self = Self::System(0b10_001_0010_1000_010);
    /// Address Comparator Access Type Register 12
    pub const TRCACATR12: Self = Self::System(0b10_001_0010_1000_011);
    /// Address Comparator Value Register 5
    pub const TRCACVR5: Self = Self::System(0b10_001_0010_1010_000);
    /// Address Comparator Value Register 13
    pub const TRCACVR13: Self = Self::System(0b10_001_0010_1010_001);
    /// Address Comparator Access Type Register 5
    pub const TRCACATR5: Self = Self::System(0b10_001_0010_1010_010);
    /// Address Comparator Access Type Register 13
    pub const TRCACATR13: Self = Self::System(0b10_001_0010_1010_011);
    /// Address Comparator Value Register 6
    pub const TRCACVR6: Self = Self::System(0b10_001_0010_1100_000);
    /// Address Comparator Value Register 14
    pub const TRCACVR14: Self = Self::System(0b10_001_0010_1100_001);
    /// Address Comparator Access Type Register 6
    pub const TRCACATR6: Self = Self::System(0b10_001_0010_1100_010);
    /// Address Comparator Access Type Register 14
    pub const TRCACATR14: Self = Self::System(0b10_001_0010_1100_011);
    /// Address Comparator Value Register 7
    pub const TRCACVR7: Self = Self::System(0b10_001_0010_1110_000);
    /// Address Comparator Value Register 15
    pub const TRCACVR15: Self = Self::System(0b10_001_0010_1110_001);
    /// Address Comparator Access Type Register 7
    pub const TRCACATR7: Self = Self::System(0b10_001_0010_1110_010);
    /// Address Comparator Access Type Register 15
    pub const TRCACATR15: Self = Self::System(0b10_001_0010_1110_011);
    /// Context Identifier Comparator Value Registers 0
    pub const TRCCIDCVR0: Self = Self::System(0b10_001_0011_0000_000);
    /// Virtual Context Identifier Comparator Value Register 0
    pub const TRCVMIDCVR0: Self = Self::System(0b10_001_0011_0000_001);
    /// Context Identifier Comparator Control Register 0
    pub const TRCCIDCCTLR0: Self = Self::System(0b10_001_0011_0000_010);
    /// Context Identifier Comparator Control Register 1
    pub const TRCCIDCCTLR1: Self = Self::System(0b10_001_0011_0001_010);
    /// Context Identifier Comparator Value Registers 1
    pub const TRCCIDCVR1: Self = Self::System(0b10_001_0011_0010_000);
    /// Virtual Context Identifier Comparator Value Register 1
    pub const TRCVMIDCVR1: Self = Self::System(0b10_001_0011_0010_001);
    /// Virtual Context Identifier Comparator Control Register 0
    pub const TRCVMIDCCTLR0: Self = Self::System(0b10_001_0011_0010_010);
    /// Virtual Context Identifier Comparator Control Register 1
    pub const TRCVMIDCCTLR1: Self = Self::System(0b10_001_0011_0011_010);
    /// Context Identifier Comparator Value Registers 2
    pub const TRCCIDCVR2: Self = Self::System(0b10_001_0011_0100_000);
    /// Virtual Context Identifier Comparator Value Register 2
    pub const TRCVMIDCVR2: Self = Self::System(0b10_001_0011_0100_001);
    /// Context Identifier Comparator Value Registers 3
    pub const TRCCIDCVR3: Self = Self::System(0b10_001_0011_0110_000);
    /// Virtual Context Identifier Comparator Value Register 3
    pub const TRCVMIDCVR3: Self = Self::System(0b10_001_0011_0110_001);
    /// Context Identifier Comparator Value Registers 4
    pub const TRCCIDCVR4: Self = Self::System(0b10_001_0011_1000_000);
    /// Virtual Context Identifier Comparator Value Register 4
    pub const TRCVMIDCVR4: Self = Self::System(0b10_001_0011_1000_001);
    /// Context Identifier Comparator Value Registers 5
    pub const TRCCIDCVR5: Self = Self::System(0b10_001_0011_1010_000);
    /// Virtual Context Identifier Comparator Value Register 5
    pub const TRCVMIDCVR5: Self = Self::System(0b10_001_0011_1010_001);
    /// Context Identifier Comparator Value Registers 6
    pub const TRCCIDCVR6: Self = Self::System(0b10_001_0011_1100_000);
    /// Virtual Context Identifier Comparator Value Register 6
    pub const TRCVMIDCVR6: Self = Self::System(0b10_001_0011_1100_001);
    /// Context Identifier Comparator Value Registers 7
    pub const TRCCIDCVR7: Self = Self::System(0b10_001_0011_1110_000);
    /// Virtual Context Identifier Comparator Value Register 7
    pub const TRCVMIDCVR7: Self = Self::System(0b10_001_0011_1110_001);
    /// Device Configuration Register
    pub const TRCDEVID: Self = Self::System(0b10_001_0111_0010_111);
    /// Claim Tag Set Register
    pub const TRCCLAIMSET: Self = Self::System(0b10_001_0111_1000_110);
    /// Claim Tag Clear Register
    pub const TRCCLAIMCLR: Self = Self::System(0b10_001_0111_1001_110);
    /// Authentication Status Register
    pub const TRCAUTHSTATUS: Self = Self::System(0b10_001_0111_1110_110);
    /// Device Architecture Register
    pub const TRCDEVARCH: Self = Self::System(0b10_001_0111_1111_110);
    /// Branch Record Buffer Information Register 0
    pub const BRBINF0_EL1: Self = Self::System(0b10_001_1000_0000_000);
    /// Branch Record Buffer Source Address Register 0
    pub const BRBSRC0_EL1: Self = Self::System(0b10_001_1000_0000_001);
    /// Branch Record Buffer Target Address Register 0
    pub const BRBTGT0_EL1: Self = Self::System(0b10_001_1000_0000_010);
    /// Branch Record Buffer Information Register 16
    pub const BRBINF16_EL1: Self = Self::System(0b10_001_1000_0000_100);
    /// Branch Record Buffer Source Address Register 16
    pub const BRBSRC16_EL1: Self = Self::System(0b10_001_1000_0000_101);
    /// Branch Record Buffer Target Address Register 16
    pub const BRBTGT16_EL1: Self = Self::System(0b10_001_1000_0000_110);
    /// Branch Record Buffer Information Register 1
    pub const BRBINF1_EL1: Self = Self::System(0b10_001_1000_0001_000);
    /// Branch Record Buffer Source Address Register 1
    pub const BRBSRC1_EL1: Self = Self::System(0b10_001_1000_0001_001);
    /// Branch Record Buffer Target Address Register 1
    pub const BRBTGT1_EL1: Self = Self::System(0b10_001_1000_0001_010);
    /// Branch Record Buffer Information Register 17
    pub const BRBINF17_EL1: Self = Self::System(0b10_001_1000_0001_100);
    /// Branch Record Buffer Source Address Register 17
    pub const BRBSRC17_EL1: Self = Self::System(0b10_001_1000_0001_101);
    /// Branch Record Buffer Target Address Register 17
    pub const BRBTGT17_EL1: Self = Self::System(0b10_001_1000_0001_110);
    /// Branch Record Buffer Information Register 2
    pub const BRBINF2_EL1: Self = Self::System(0b10_001_1000_0010_000);
    /// Branch Record Buffer Source Address Register 2
    pub const BRBSRC2_EL1: Self = Self::System(0b10_001_1000_0010_001);
    /// Branch Record Buffer Target Address Register 2
    pub const BRBTGT2_EL1: Self = Self::System(0b10_001_1000_0010_010);
    /// Branch Record Buffer Information Register 18
    pub const BRBINF18_EL1: Self = Self::System(0b10_001_1000_0010_100);
    /// Branch Record Buffer Source Address Register 18
    pub const BRBSRC18_EL1: Self = Self::System(0b10_001_1000_0010_101);
    /// Branch Record Buffer Target Address Register 18
    pub const BRBTGT18_EL1: Self = Self::System(0b10_001_1000_0010_110);
    /// Branch Record Buffer Information Register 3
    pub const BRBINF3_EL1: Self = Self::System(0b10_001_1000_0011_000);
    /// Branch Record Buffer Source Address Register 3
    pub const BRBSRC3_EL1: Self = Self::System(0b10_001_1000_0011_001);
    /// Branch Record Buffer Target Address Register 3
    pub const BRBTGT3_EL1: Self = Self::System(0b10_001_1000_0011_010);
    /// Branch Record Buffer Information Register 19
    pub const BRBINF19_EL1: Self = Self::System(0b10_001_1000_0011_100);
    /// Branch Record Buffer Source Address Register 19
    pub const BRBSRC19_EL1: Self = Self::System(0b10_001_1000_0011_101);
    /// Branch Record Buffer Target Address Register 19
    pub const BRBTGT19_EL1: Self = Self::System(0b10_001_1000_0011_110);
    /// Branch Record Buffer Information Register 4
    pub const BRBINF4_EL1: Self = Self::System(0b10_001_1000_0100_000);
    /// Branch Record Buffer Source Address Register 4
    pub const BRBSRC4_EL1: Self = Self::System(0b10_001_1000_0100_001);
    /// Branch Record Buffer Target Address Register 4
    pub const BRBTGT4_EL1: Self = Self::System(0b10_001_1000_0100_010);
    /// Branch Record Buffer Information Register 20
    pub const BRBINF20_EL1: Self = Self::System(0b10_001_1000_0100_100);
    /// Branch Record Buffer Source Address Register 20
    pub const BRBSRC20_EL1: Self = Self::System(0b10_001_1000_0100_101);
    /// Branch Record Buffer Target Address Register 20
    pub const BRBTGT20_EL1: Self = Self::System(0b10_001_1000_0100_110);
    /// Branch Record Buffer Information Register 5
    pub const BRBINF5_EL1: Self = Self::System(0b10_001_1000_0101_000);
    /// Branch Record Buffer Source Address Register 5
    pub const BRBSRC5_EL1: Self = Self::System(0b10_001_1000_0101_001);
    /// Branch Record Buffer Target Address Register 5
    pub const BRBTGT5_EL1: Self = Self::System(0b10_001_1000_0101_010);
    /// Branch Record Buffer Information Register 21
    pub const BRBINF21_EL1: Self = Self::System(0b10_001_1000_0101_100);
    /// Branch Record Buffer Source Address Register 21
    pub const BRBSRC21_EL1: Self = Self::System(0b10_001_1000_0101_101);
    /// Branch Record Buffer Target Address Register 21
    pub const BRBTGT21_EL1: Self = Self::System(0b10_001_1000_0101_110);
    /// Branch Record Buffer Information Register 6
    pub const BRBINF6_EL1: Self = Self::System(0b10_001_1000_0110_000);
    /// Branch Record Buffer Source Address Register 6
    pub const BRBSRC6_EL1: Self = Self::System(0b10_001_1000_0110_001);
    /// Branch Record Buffer Target Address Register 6
    pub const BRBTGT6_EL1: Self = Self::System(0b10_001_1000_0110_010);
    /// Branch Record Buffer Information Register 22
    pub const BRBINF22_EL1: Self = Self::System(0b10_001_1000_0110_100);
    /// Branch Record Buffer Source Address Register 22
    pub const BRBSRC22_EL1: Self = Self::System(0b10_001_1000_0110_101);
    /// Branch Record Buffer Target Address Register 22
    pub const BRBTGT22_EL1: Self = Self::System(0b10_001_1000_0110_110);
    /// Branch Record Buffer Information Register 7
    pub const BRBINF7_EL1: Self = Self::System(0b10_001_1000_0111_000);
    /// Branch Record Buffer Source Address Register 7
    pub const BRBSRC7_EL1: Self = Self::System(0b10_001_1000_0111_001);
    /// Branch Record Buffer Target Address Register 7
    pub const BRBTGT7_EL1: Self = Self::System(0b10_001_1000_0111_010);
    /// Branch Record Buffer Information Register 23
    pub const BRBINF23_EL1: Self = Self::System(0b10_001_1000_0111_100);
    /// Branch Record Buffer Source Address Register 23
    pub const BRBSRC23_EL1: Self = Self::System(0b10_001_1000_0111_101);
    /// Branch Record Buffer Target Address Register 23
    pub const BRBTGT23_EL1: Self = Self::System(0b10_001_1000_0111_110);
    /// Branch Record Buffer Information Register 8
    pub const BRBINF8_EL1: Self = Self::System(0b10_001_1000_1000_000);
    /// Branch Record Buffer Source Address Register 8
    pub const BRBSRC8_EL1: Self = Self::System(0b10_001_1000_1000_001);
    /// Branch Record Buffer Target Address Register 8
    pub const BRBTGT8_EL1: Self = Self::System(0b10_001_1000_1000_010);
    /// Branch Record Buffer Information Register 24
    pub const BRBINF24_EL1: Self = Self::System(0b10_001_1000_1000_100);
    /// Branch Record Buffer Source Address Register 24
    pub const BRBSRC24_EL1: Self = Self::System(0b10_001_1000_1000_101);
    /// Branch Record Buffer Target Address Register 24
    pub const BRBTGT24_EL1: Self = Self::System(0b10_001_1000_1000_110);
    /// Branch Record Buffer Information Register 9
    pub const BRBINF9_EL1: Self = Self::System(0b10_001_1000_1001_000);
    /// Branch Record Buffer Source Address Register 9
    pub const BRBSRC9_EL1: Self = Self::System(0b10_001_1000_1001_001);
    /// Branch Record Buffer Target Address Register 9
    pub const BRBTGT9_EL1: Self = Self::System(0b10_001_1000_1001_010);
    /// Branch Record Buffer Information Register 25
    pub const BRBINF25_EL1: Self = Self::System(0b10_001_1000_1001_100);
    /// Branch Record Buffer Source Address Register 25
    pub const BRBSRC25_EL1: Self = Self::System(0b10_001_1000_1001_101);
    /// Branch Record Buffer Target Address Register 25
    pub const BRBTGT25_EL1: Self = Self::System(0b10_001_1000_1001_110);
    /// Branch Record Buffer Information Register 10
    pub const BRBINF10_EL1: Self = Self::System(0b10_001_1000_1010_000);
    /// Branch Record Buffer Source Address Register 10
    pub const BRBSRC10_EL1: Self = Self::System(0b10_001_1000_1010_001);
    /// Branch Record Buffer Target Address Register 10
    pub const BRBTGT10_EL1: Self = Self::System(0b10_001_1000_1010_010);
    /// Branch Record Buffer Information Register 26
    pub const BRBINF26_EL1: Self = Self::System(0b10_001_1000_1010_100);
    /// Branch Record Buffer Source Address Register 26
    pub const BRBSRC26_EL1: Self = Self::System(0b10_001_1000_1010_101);
    /// Branch Record Buffer Target Address Register 26
    pub const BRBTGT26_EL1: Self = Self::System(0b10_001_1000_1010_110);
    /// Branch Record Buffer Information Register 11
    pub const BRBINF11_EL1: Self = Self::System(0b10_001_1000_1011_000);
    /// Branch Record Buffer Source Address Register 11
    pub const BRBSRC11_EL1: Self = Self::System(0b10_001_1000_1011_001);
    /// Branch Record Buffer Target Address Register 11
    pub const BRBTGT11_EL1: Self = Self::System(0b10_001_1000_1011_010);
    /// Branch Record Buffer Information Register 27
    pub const BRBINF27_EL1: Self = Self::System(0b10_001_1000_1011_100);
    /// Branch Record Buffer Source Address Register 27
    pub const BRBSRC27_EL1: Self = Self::System(0b10_001_1000_1011_101);
    /// Branch Record Buffer Target Address Register 27
    pub const BRBTGT27_EL1: Self = Self::System(0b10_001_1000_1011_110);
    /// Branch Record Buffer Information Register 12
    pub const BRBINF12_EL1: Self = Self::System(0b10_001_1000_1100_000);
    /// Branch Record Buffer Source Address Register 12
    pub const BRBSRC12_EL1: Self = Self::System(0b10_001_1000_1100_001);
    /// Branch Record Buffer Target Address Register 12
    pub const BRBTGT12_EL1: Self = Self::System(0b10_001_1000_1100_010);
    /// Branch Record Buffer Information Register 28
    pub const BRBINF28_EL1: Self = Self::System(0b10_001_1000_1100_100);
    /// Branch Record Buffer Source Address Register 28
    pub const BRBSRC28_EL1: Self = Self::System(0b10_001_1000_1100_101);
    /// Branch Record Buffer Target Address Register 28
    pub const BRBTGT28_EL1: Self = Self::System(0b10_001_1000_1100_110);
    /// Branch Record Buffer Information Register 13
    pub const BRBINF13_EL1: Self = Self::System(0b10_001_1000_1101_000);
    /// Branch Record Buffer Source Address Register 13
    pub const BRBSRC13_EL1: Self = Self::System(0b10_001_1000_1101_001);
    /// Branch Record Buffer Target Address Register 13
    pub const BRBTGT13_EL1: Self = Self::System(0b10_001_1000_1101_010);
    /// Branch Record Buffer Information Register 29
    pub const BRBINF29_EL1: Self = Self::System(0b10_001_1000_1101_100);
    /// Branch Record Buffer Source Address Register 29
    pub const BRBSRC29_EL1: Self = Self::System(0b10_001_1000_1101_101);
    /// Branch Record Buffer Target Address Register 29
    pub const BRBTGT29_EL1: Self = Self::System(0b10_001_1000_1101_110);
    /// Branch Record Buffer Information Register 14
    pub const BRBINF14_EL1: Self = Self::System(0b10_001_1000_1110_000);
    /// Branch Record Buffer Source Address Register 14
    pub const BRBSRC14_EL1: Self = Self::System(0b10_001_1000_1110_001);
    /// Branch Record Buffer Target Address Register 14
    pub const BRBTGT14_EL1: Self = Self::System(0b10_001_1000_1110_010);
    /// Branch Record Buffer Information Register 30
    pub const BRBINF30_EL1: Self = Self::System(0b10_001_1000_1110_100);
    /// Branch Record Buffer Source Address Register 30
    pub const BRBSRC30_EL1: Self = Self::System(0b10_001_1000_1110_101);
    /// Branch Record Buffer Target Address Register 30
    pub const BRBTGT30_EL1: Self = Self::System(0b10_001_1000_1110_110);
    /// Branch Record Buffer Information Register 15
    pub const BRBINF15_EL1: Self = Self::System(0b10_001_1000_1111_000);
    /// Branch Record Buffer Source Address Register 15
    pub const BRBSRC15_EL1: Self = Self::System(0b10_001_1000_1111_001);
    /// Branch Record Buffer Target Address Register 15
    pub const BRBTGT15_EL1: Self = Self::System(0b10_001_1000_1111_010);
    /// Branch Record Buffer Information Register 31
    pub const BRBINF31_EL1: Self = Self::System(0b10_001_1000_1111_100);
    /// Branch Record Buffer Source Address Register 31
    pub const BRBSRC31_EL1: Self = Self::System(0b10_001_1000_1111_101);
    /// Branch Record Buffer Target Address Register 31
    pub const BRBTGT31_EL1: Self = Self::System(0b10_001_1000_1111_110);
    /// Branch Record Buffer Control Register (EL1)
    pub const BRBCR_EL1: Self = Self::System(0b10_001_1001_0000_000);
    /// Branch Record Buffer Control Register (EL2)
    pub const BRBCR_EL2: Self = Self::System(0b10_001_1001_0000_000);
    /// Branch Record Buffer Function Control Register
    pub const BRBFCR_EL1: Self = Self::System(0b10_001_1001_0000_001);
    /// Branch Record Buffer Timestamp Register
    pub const BRBTS_EL1: Self = Self::System(0b10_001_1001_0000_010);
    /// Branch Record Buffer Information Injection Register
    pub const BRBINFINJ_EL1: Self = Self::System(0b10_001_1001_0001_000);
    /// Branch Record Buffer Source Address Injection Register
    pub const BRBSRCINJ_EL1: Self = Self::System(0b10_001_1001_0001_001);
    /// Branch Record Buffer Target Address Injection Register
    pub const BRBTGTINJ_EL1: Self = Self::System(0b10_001_1001_0001_010);
    /// Branch Record Buffer ID0 Register
    pub const BRBIDR0_EL1: Self = Self::System(0b10_001_1001_0010_000);
    /// Monitor DCC Status Register
    pub const MDCCSR_EL0: Self = Self::System(0b10_011_0000_0001_000);
    /// Debug Data Transfer Register, Half-duplex
    pub const DBGDTR_EL0: Self = Self::System(0b10_011_0000_0100_000);
    /// Debug Data Transfer Register, Receive
    pub const DBGDTRRX_EL0: Self = Self::System(0b10_011_0000_0101_000);
    /// Debug Data Transfer Register, Transmit
    pub const DBGDTRTX_EL0: Self = Self::System(0b10_011_0000_0101_000);
    /// Debug Vector Catch Register
    pub const DBGVCR32_EL2: Self = Self::System(0b10_100_0000_0111_000);
}
