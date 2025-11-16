use super::prelude::*;
use crate::arch::Arch;
use crate::protocol::commands::ext::XLowcasePacket;
use crate::target::ext::base::BaseOps;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_x_lowcase_packet(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: XLowcasePacket<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        if !target.use_x_lowcase_packet() {
            return Ok(HandlerStatus::Handled);
        }

        crate::__dead_code_marker!("x_lowcase_packet", "impl");

        let handler_status = match command {
            XLowcasePacket::x(cmd) => {
                let buf = cmd.buf;
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                let mut i = 0;
                let mut n = cmd.len;
                while n != 0 {
                    let chunk_size = n.min(buf.len());

                    use num_traits::NumCast;

                    let addr = addr + NumCast::from(i).ok_or(Error::TargetMismatch)?;
                    let data = &mut buf[..chunk_size];
                    let data_len = match target.base_ops() {
                        BaseOps::SingleThread(ops) => ops.read_addrs(addr, data),
                        BaseOps::MultiThread(ops) => {
                            ops.read_addrs(addr, data, self.current_mem_tid)
                        }
                    }
                    .handle_error()?;

                    n -= chunk_size;
                    i += chunk_size;

                    // TODO: add more specific error variant?
                    let data = data.get(..data_len).ok_or(Error::PacketBufferOverflow)?;
                    if i == 0 {
                        res.write_binary(b"b")?;
                    }
                    res.write_binary(data)?;
                }

                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
