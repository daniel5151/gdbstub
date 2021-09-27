use super::prelude::*;
use crate::protocol::commands::ext::SingleRegisterAccess;

use crate::arch::{Arch, RegId};
use crate::target::ext::base::BaseOps;

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
                let mut buf = p.buf;
                if let Some(size) = reg_size {
                    buf = &mut buf[0..size.get()];
                }

                let len = ops.read_register(id, reg_id, buf).handle_error()?;

                if let Some(size) = reg_size {
                    if size.get() != len {
                        return Err(Error::TargetMismatch);
                    }
                } else {
                    buf = &mut buf[0..len];
                }
                res.write_hex_buf(buf)?;
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
