use super::prelude::*;
use crate::common::Endianness;
use crate::protocol::commands::ext::ProcessInfo;
use crate::protocol::ResponseWriterError;
use crate::target::ext::process_info::InfoResponse;

impl<'a> InfoResponse<'a> {
    fn write_response<C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), ResponseWriterError<C::Error>> {
        match self {
            InfoResponse::Pid(pid) => {
                res.write_str("pid:")?;
                res.write_dec(usize::from(*pid))?;
            }
            InfoResponse::Triple(triple) => {
                res.write_str("triple:")?;
                res.write_hex_buf(triple.as_bytes())?;
            }
            InfoResponse::Endianness(endian) => {
                res.write_str("endian:")?;
                res.write_str(match endian {
                    Endianness::Big => "big;",
                    Endianness::Little => "little;",
                })?;
            }
            InfoResponse::PointerSize(p) => {
                res.write_str("ptrsize:")?;
                res.write_dec(*p)?;
            }
        }
        res.write_str(";")?;
        Ok(())
    }
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_process_info(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: ProcessInfo,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_process_info() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("process_info", "impl");

        let mut result = Ok(());
        let mut write_info = |info: &InfoResponse<'_>| {
            if result.is_ok() {
                if let Err(e) = info.write_response(res) {
                    result = Err(e);
                }
            }
        };

        let handler_status = match command {
            ProcessInfo::qHostInfo(_cmd) => {
                ops.host_info(&mut write_info).map_err(Error::TargetError)?;
                result?;
                HandlerStatus::Handled
            }
            ProcessInfo::qProcessInfo(_cmd) => {
                ops.process_info(&mut write_info)
                    .map_err(Error::TargetError)?;
                result?;
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
