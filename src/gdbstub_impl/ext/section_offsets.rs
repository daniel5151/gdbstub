use super::prelude::*;
use crate::protocol::commands::ext::SectionOffsets;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_section_offsets(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: SectionOffsets,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.section_offsets() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let handler_status = match command {
            SectionOffsets::qOffsets(_cmd) => {
                use crate::target::ext::section_offsets::Offsets;

                crate::__dead_code_marker!("qOffsets", "impl");

                match ops.get_section_offsets().map_err(Error::TargetError)? {
                    Offsets::Sections { text, data, bss } => {
                        res.write_str("Text=")?;
                        res.write_num(text)?;

                        res.write_str(";Data=")?;
                        res.write_num(data)?;

                        // "Note: while a Bss offset may be included in the response,
                        // GDB ignores this and instead applies the Data offset to the Bss section."
                        //
                        // While this would suggest that it's OK to omit `Bss=` entirely, recent
                        // versions of GDB seem to require that `Bss=` is present.
                        //
                        // See https://github.com/bminor/binutils-gdb/blob/master/gdb/remote.c#L4149-L4159
                        let bss = bss.unwrap_or(data);
                        res.write_str(";Bss=")?;
                        res.write_num(bss)?;
                    }
                    Offsets::Segments { text_seg, data_seg } => {
                        res.write_str("TextSeg=")?;
                        res.write_num(text_seg)?;

                        if let Some(data) = data_seg {
                            res.write_str(";DataSeg=")?;
                            res.write_num(data)?;
                        }
                    }
                }
                HandlerStatus::Handled
            }
        };

        Ok(handler_status)
    }
}
