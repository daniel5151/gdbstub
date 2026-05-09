use super::prelude::*;
use crate::arch::Arch;
use crate::arch::RegId;
use crate::protocol::commands::ext::SingleRegisterAccess;
use crate::target::ext::base::BaseOps;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_single_register_access(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: SingleRegisterAccess<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match match target.base_ops() {
            BaseOps::SingleThread(ops) => ops.support_single_register_access(),
            BaseOps::MultiThread(ops) => ops.support_single_register_access(),
        } {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let handler_status = match command {
            SingleRegisterAccess::p(p) => {
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                let (reg_id, reg_size) = match reg {
                    None => {
                        warn!("reg id {} does not map onto any known register", p.reg_id);
                        return Ok(HandlerStatus::Handled);
                    }
                    Some(v) => v,
                };
                let mut buf = p.buf;
                if let Some(size) = reg_size {
                    buf = buf
                        .get_mut(..size.get())
                        .ok_or(Error::PacketBufferOverflow)?;
                }

                let len = ops
                    .read_register(self.current_mem_tid, reg_id, buf)
                    .handle_error()?;

                if len == 0 {
                    if let Some(size) = reg_size {
                        for _ in 0..size.get() {
                            res.write_str("xx")?;
                        }
                    } else {
                        return Err(Error::UnexpectedReg);
                    }
                } else {
                    if let Some(size) = reg_size {
                        if size.get() != len {
                            return Err(Error::UnexpectedReg);
                        }
                    } else {
                        buf = buf.get_mut(..len).ok_or(Error::PacketBufferOverflow)?;
                    }
                    res.write_hex_buf(buf)?;
                }
                HandlerStatus::Handled
            }
            SingleRegisterAccess::P(p) => {
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                match reg {
                    // empty packet indicates unrecognized query
                    None => return Ok(HandlerStatus::Handled),
                    Some((reg_id, _)) => ops
                        .write_register(self.current_mem_tid, reg_id, p.val)
                        .handle_error()?,
                }
                HandlerStatus::NeedsOk
            }
        };

        Ok(handler_status)
    }
}
