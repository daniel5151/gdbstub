use super::prelude::*;
use crate::common::Endianness;
use crate::common::Pid;
use crate::protocol::ResponseWriterError;

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

impl<'a> From<&crate::target::ext::host_info::InfoResponse<'a>> for InfoResponse<'a> {
    fn from(resp: &crate::target::ext::host_info::InfoResponse<'a>) -> Self {
        use crate::target::ext::host_info::InfoResponse as R;
        match *resp {
            R::Triple(s) => InfoResponse::Triple(s),
            R::Endianness(e) => InfoResponse::Endianness(e),
            R::PointerSize(p) => InfoResponse::PointerSize(p),
        }
    }
}

impl<'a> From<&crate::target::ext::process_info::InfoResponse<'a>> for InfoResponse<'a> {
    fn from(resp: &crate::target::ext::process_info::InfoResponse<'a>) -> Self {
        use crate::target::ext::process_info::InfoResponse as R;
        match *resp {
            R::Pid(pid) => InfoResponse::Pid(pid),
            R::Triple(s) => InfoResponse::Triple(s),
            R::Endianness(e) => InfoResponse::Endianness(e),
            R::PointerSize(p) => InfoResponse::PointerSize(p),
        }
    }
}
