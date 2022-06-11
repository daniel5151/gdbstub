use super::prelude::*;
use crate::protocol::commands::ext::RegisterInfo;

use crate::arch::lldb::{Encoding, Format, Generic, Register, RegisterInfo as LLDBRegisterInfo};
use crate::arch::Arch;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_register_info(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: RegisterInfo,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        if !target.use_register_info() {
            return Ok(HandlerStatus::Handled);
        }

        let handler_status = match command {
            RegisterInfo::qRegisterInfo(cmd) => {
                let mut err = Ok(());
                let cb = &mut |reg: Option<Register<'_>>| {
                    let res = match reg {
                        // TODO: replace this with a try block (once stabilized)
                        Some(reg) => (|| {
                            res.write_str("name:")?;
                            res.write_str(reg.name)?;
                            if let Some(alt_name) = reg.alt_name {
                                res.write_str(";alt-name:")?;
                                res.write_str(alt_name)?;
                            }
                            res.write_str(";bitsize:")?;
                            res.write_dec(reg.bitsize)?;
                            res.write_str(";offset:")?;
                            res.write_dec(reg.offset)?;
                            res.write_str(";encoding:")?;
                            res.write_str(match reg.encoding {
                                Encoding::Uint => "uint",
                                Encoding::Sint => "sint",
                                Encoding::IEEE754 => "ieee754",
                                Encoding::Vector => "vector",
                            })?;
                            res.write_str(";format:")?;
                            res.write_str(match reg.format {
                                Format::Binary => "binary",
                                Format::Decimal => "decimal",
                                Format::Hex => "hex",
                                Format::Float => "float",
                                Format::VectorSInt8 => "vector-sint8",
                                Format::VectorUInt8 => "vector-uint8",
                                Format::VectorSInt16 => "vector-sint16",
                                Format::VectorUInt16 => "vector-uint16",
                                Format::VectorSInt32 => "vector-sint32",
                                Format::VectorUInt32 => "vector-uint32",
                                Format::VectorFloat32 => "vector-float32",
                                Format::VectorUInt128 => "vector-uint128",
                            })?;
                            res.write_str(";set:")?;
                            res.write_str(reg.set)?;
                            if let Some(gcc) = reg.gcc {
                                res.write_str(";gcc:")?;
                                res.write_dec(gcc)?;
                            }
                            if let Some(dwarf) = reg.dwarf {
                                res.write_str(";dwarf:")?;
                                res.write_dec(dwarf)?;
                            }
                            if let Some(generic) = reg.generic {
                                res.write_str(";generic:")?;
                                res.write_str(match generic {
                                    Generic::Pc => "pc",
                                    Generic::Sp => "sp",
                                    Generic::Fp => "fp",
                                    Generic::Ra => "ra",
                                    Generic::Flags => "flags",
                                    Generic::Arg1 => "arg1",
                                    Generic::Arg2 => "arg2",
                                    Generic::Arg3 => "arg3",
                                    Generic::Arg4 => "arg4",
                                    Generic::Arg5 => "arg5",
                                    Generic::Arg6 => "arg6",
                                    Generic::Arg7 => "arg7",
                                    Generic::Arg8 => "arg8",
                                })?;
                            }
                            if let Some(c_regs) = reg.container_regs {
                                res.write_str(";container-regs:")?;
                                res.write_num(c_regs[0])?;
                                for reg in c_regs.iter().skip(1) {
                                    res.write_str(",")?;
                                    res.write_num(*reg)?;
                                }
                            }
                            if let Some(i_regs) = reg.invalidate_regs {
                                res.write_str(";invalidate-regs:")?;
                                res.write_num(i_regs[0])?;
                                for reg in i_regs.iter().skip(1) {
                                    res.write_str(",")?;
                                    res.write_num(*reg)?;
                                }
                            }
                            res.write_str(";")
                        })(),
                        // In fact, this doesn't has to be E45! It could equally well be any
                        // other error code or even an eOk, eAck or eNack! It turns out that
                        // 0x45 == 69, so presumably the LLDB people were just having some fun
                        // here. For a little discussion on this and LLDB source code pointers,
                        // see https://github.com/daniel5151/gdbstub/pull/103#discussion_r888590197
                        _ => res.write_str("E45"),
                    };
                    if let Err(e) = res {
                        err = Err(e);
                    }
                };
                if let Some(ops) = target.support_register_info_override() {
                    use crate::target::ext::register_info_override::{Callback, CallbackToken};

                    ops.register_info(
                        cmd.reg_id,
                        Callback {
                            cb,
                            token: CallbackToken(core::marker::PhantomData),
                        },
                    )
                    .map_err(Error::TargetError)?;
                    err?;
                } else if let Some(reg) = T::Arch::register_info(cmd.reg_id) {
                    match reg {
                        LLDBRegisterInfo::Register(reg) => cb(Some(reg)),
                        LLDBRegisterInfo::Done => cb(None),
                    };
                }
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
