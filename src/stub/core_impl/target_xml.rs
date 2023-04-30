use super::prelude::*;
use crate::arch::Arch;
use crate::protocol::commands::ext::TargetXml;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_target_xml(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: TargetXml<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        if !target.use_target_description_xml() {
            return Ok(HandlerStatus::Handled);
        }

        let handler_status = match command {
            TargetXml::qXferFeaturesRead(cmd) => {
                let ret = if let Some(ops) = target.support_target_description_xml_override() {
                    ops.target_description_xml(cmd.annex.name, cmd.offset, cmd.length, cmd.buf)
                        .handle_error()?
                } else if let Some(xml) = T::Arch::target_description_xml() {
                    if cmd.annex.name != b"target.xml" {
                        // TODO: not the best error... should probably report to the user the
                        // <xi:include> isn't supported at the Arch level (yet)
                        return Err(Error::PacketUnexpected);
                    }

                    let xml = xml.trim().as_bytes();
                    let xml_len = xml.len();

                    let start = xml_len.min(cmd.offset as usize);
                    let end = xml_len.min((cmd.offset as usize).saturating_add(cmd.length));

                    // LLVM isn't smart enough to realize that `start <= end`, and fails to elide a
                    // `slice_end_index_len_fail` check unless we include this seemingly useless
                    // call to `min`.
                    let data = &xml[start.min(end)..end];

                    let n = data.len().min(cmd.buf.len());
                    cmd.buf[..n].copy_from_slice(&data[..n]);
                    n
                } else {
                    // If the target hasn't provided their own XML, then the initial response to
                    // "qSupported" wouldn't have included "qXfer:features:read", and gdb wouldn't
                    // send this packet unless it was explicitly marked as supported.
                    return Err(Error::PacketUnexpected);
                };

                if ret == 0 {
                    res.write_str("l")?;
                } else {
                    res.write_str("m")?;
                    // TODO: add more specific error variant?
                    res.write_binary(cmd.buf.get(..ret).ok_or(Error::PacketBufferOverflow)?)?;
                }
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
