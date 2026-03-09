use super::prelude::*;
use crate::common::Endianness;
use crate::common::Pid;
use crate::protocol::commands::ext::HostInfo;
use crate::protocol::commands::ext::ProcessInfo;
use crate::protocol::ResponseWriterError;
use crate::target::ext::host_info::HostInfoResponse;
use crate::target::ext::process_info::ProcessInfoResponse;

pub(crate) enum InfoResponse<'a> {
    Pid(Pid),
    Triple(&'a str),
    Endianness(Endianness),
    PointerSize(usize),
}

impl<'a> InfoResponse<'a> {
    pub(crate) fn write_response<C: Connection>(
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

impl<'a> From<&HostInfoResponse<'a>> for InfoResponse<'a> {
    fn from(resp: &HostInfoResponse<'a>) -> Self {
        match *resp {
            HostInfoResponse::Triple(s) => InfoResponse::Triple(s),
            HostInfoResponse::Endianness(e) => InfoResponse::Endianness(e),
            HostInfoResponse::PointerSize(p) => InfoResponse::PointerSize(p),
        }
    }
}

impl<'a> From<&ProcessInfoResponse<'a>> for InfoResponse<'a> {
    fn from(resp: &ProcessInfoResponse<'a>) -> Self {
        match *resp {
            ProcessInfoResponse::Pid(pid) => InfoResponse::Pid(pid),
            ProcessInfoResponse::Triple(s) => InfoResponse::Triple(s),
            ProcessInfoResponse::Endianness(e) => InfoResponse::Endianness(e),
            ProcessInfoResponse::PointerSize(p) => InfoResponse::PointerSize(p),
        }
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
        let mut write_info = |info: &ProcessInfoResponse<'_>| {
            if result.is_ok() {
                if let Err(e) = InfoResponse::from(info).write_response(res) {
                    result = Err(e);
                }
            }
        };

        match command {
            ProcessInfo::qProcessInfo(_cmd) => {
                ops.process_info(&mut write_info)
                    .map_err(Error::TargetError)?;
                result?;
            }
        };

        Ok(HandlerStatus::Handled)
    }

    pub(crate) fn handle_host_info(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: HostInfo,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_host_info() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("host_info", "impl");

        let mut result = Ok(());
        let mut write_info = |info: &HostInfoResponse<'_>| {
            if result.is_ok() {
                if let Err(e) = InfoResponse::from(info).write_response(res) {
                    result = Err(e);
                }
            }
        };

        match command {
            HostInfo::qHostInfo(_cmd) => {
                ops.host_info(&mut write_info).map_err(Error::TargetError)?;
                result?;
            }
        };

        Ok(HandlerStatus::Handled)
    }
}
