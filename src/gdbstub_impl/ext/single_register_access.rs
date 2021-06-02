use super::prelude::*;
use crate::protocol::commands::ext::SingleRegisterAccess;

use crate::arch::{Arch, RegId};
use crate::target::ext::base::{BaseOps, SendRegisterOutput};

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    fn inner<Id>(
        res: &mut ResponseWriter<C>,
        ops: crate::target::ext::base::SingleRegisterAccessOps<Id, T>,
        command: SingleRegisterAccess<'_>,
        id: Id,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let handler_status = match command {
            SingleRegisterAccess::p(p) => {
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                let (reg_id, reg_size) = match reg {
                    // empty packet indicates unrecognized query
                    None => return Ok(HandlerStatus::Handled),
                    Some(v) => v,
                };

                let mut n = 0usize;
                let mut err = Ok(());

                ops.read_register(
                    id,
                    reg_id,
                    SendRegisterOutput::new(&mut |buf| {
                        if err.is_ok() {
                            // If the register has a known size and read_register attempts
                            // to send more bytes than are present in the register,
                            // error out and stop sending data.
                            if let Some(size) = reg_size {
                                n += buf.len();
                                
                                if n > size.get() {
                                    err = Err(Error::TargetMismatch);
                                    return;
                                }
                            }

                            err = res.write_hex_buf(buf).map_err(|e| e.into());
                        }
                    }),
                )
                .handle_error()?;

                err?;

                HandlerStatus::Handled
            }
            SingleRegisterAccess::P(p) => {
                let reg = <T::Arch as Arch>::RegId::from_raw_id(p.reg_id);
                match reg {
                    // empty packet indicates unrecognized query
                    None => return Ok(HandlerStatus::Handled),
                    Some((reg_id, _)) => ops.write_register(id, reg_id, p.val).handle_error()?,
                }
                HandlerStatus::NeedsOk
            }
        };

        Ok(handler_status)
    }

    pub(crate) fn handle_single_register_access<'a>(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: SingleRegisterAccess<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        match target.base_ops() {
            BaseOps::SingleThread(ops) => match ops.single_register_access() {
                None => Ok(HandlerStatus::Handled),
                Some(ops) => Self::inner(res, ops, command, ()),
            },
            BaseOps::MultiThread(ops) => match ops.single_register_access() {
                None => Ok(HandlerStatus::Handled),
                Some(ops) => Self::inner(res, ops, command, self.current_mem_tid),
            },
        }
    }
}
