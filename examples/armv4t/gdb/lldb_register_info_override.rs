use gdbstub::arch::lldb::{Encoding, Format, Generic, Register};
use gdbstub::arch::RegId;
use gdbstub::target;
use gdbstub::target::ext::lldb_register_info_override::{Callback, CallbackToken};
use gdbstub_arch::arm::reg::id::ArmCoreRegId;

use crate::gdb::custom_arch::ArmCoreRegIdCustom;
use crate::gdb::Emu;

// (LLDB extension) This implementation is for illustrative purposes only.
//
// Note: In this implementation, we have r0-pc from 0-16 but cpsr is at offset
// 25*4 in the 'g'/'G' packets, so we add 8 padding registers here. Please see
// gdbstub/examples/armv4t/gdb/target_description_xml_override.rs for more info.
impl target::ext::lldb_register_info_override::LldbRegisterInfoOverride for Emu {
    fn lldb_register_info<'a>(
        &mut self,
        reg_id: usize,
        reg_info: Callback<'a>,
    ) -> Result<CallbackToken<'a>, Self::Error> {
        // Fix for missing 24 => Self::Fps in ArmCoreRegId::from_raw_id
        let id = if reg_id == 24 { 23 } else { reg_id };

        match ArmCoreRegIdCustom::from_raw_id(id) {
            Some((_, None)) | None => Ok(reg_info.done()),
            Some((r, Some(size))) => {
                let name: String = match r {
                    // For the purpose of demonstration, we end the qRegisterInfo packet exchange
                    // when reaching the Time register id, so that this register can only be
                    // explicitly queried via the single-register read packet.
                    ArmCoreRegIdCustom::Time => return Ok(reg_info.done()),
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(i)) => match i {
                        0 => "r0",
                        1 => "r1",
                        2 => "r2",
                        3 => "r3",
                        4 => "r4",
                        5 => "r5",
                        6 => "r6",
                        7 => "r7",
                        8 => "r8",
                        9 => "r9",
                        10 => "r10",
                        11 => "r11",
                        12 => "r12",
                        _ => "unknown",
                    },
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp) => "sp",
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Lr) => "lr",
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc) => "pc",
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Fpr(_i)) => "padding",
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Fps) => "padding",
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr) => "cpsr",
                    ArmCoreRegIdCustom::Custom => "custom",
                    _ => "unknown",
                }
                .into();
                let encoding = match r {
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(_i)) => Encoding::Uint,
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp)
                    | ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc)
                    | ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr)
                    | ArmCoreRegIdCustom::Custom => Encoding::Uint,
                    _ => Encoding::Vector,
                };
                let format = match r {
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(_i)) => Format::Hex,
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp)
                    | ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc)
                    | ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr)
                    | ArmCoreRegIdCustom::Custom => Format::Hex,
                    _ => Format::VectorUInt8,
                };
                let set: String = match r {
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Gpr(_i)) => "General Purpose Registers",
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp)
                    | ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc)
                    | ArmCoreRegIdCustom::Core(ArmCoreRegId::Cpsr)
                    | ArmCoreRegIdCustom::Custom => "General Purpose Registers",
                    _ => "Floating Point Registers",
                }
                .into();
                let generic = match r {
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Sp) => Some(Generic::Sp),
                    ArmCoreRegIdCustom::Core(ArmCoreRegId::Pc) => Some(Generic::Pc),
                    _ => None,
                };
                let reg = Register {
                    name: &name,
                    alt_name: None,
                    bitsize: (usize::from(size)) * 8,
                    offset: id * (usize::from(size)),
                    encoding,
                    format,
                    set: &set,
                    gcc: None,
                    dwarf: Some(id),
                    generic,
                    container_regs: None,
                    invalidate_regs: None,
                };
                Ok(reg_info.write(reg))
            }
        }
    }
}
