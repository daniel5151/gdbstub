use super::prelude::*;
use crate::protocol::commands::ext::ReverseCont;
use crate::protocol::commands::ext::ReverseStep;
use crate::protocol::SpecificIdKind;
use crate::target::ext::base::ResumeOps;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_reverse_cont(
        &mut self,
        _res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: ReverseCont,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.base_ops().resume_ops().and_then(|ops| match ops {
            ResumeOps::SingleThread(ops) => ops.support_reverse_cont(),
            ResumeOps::MultiThread(ops) => ops.support_reverse_cont(),
        }) {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("reverse_cont", "impl");

        let handler_status = match command {
            ReverseCont::bc(_) => {
                ops.reverse_cont().map_err(Error::TargetError)?;
                HandlerStatus::DoResume
            }
        };

        Ok(handler_status)
    }

    pub(crate) fn handle_reverse_step(
        &mut self,
        _res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: ReverseStep,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.base_ops().resume_ops().and_then(|ops| match ops {
            ResumeOps::SingleThread(ops) => ops.support_reverse_step(),
            ResumeOps::MultiThread(ops) => ops.support_reverse_step(),
        }) {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("reverse_step", "impl");

        let handler_status = match command {
            ReverseStep::bs(_) => {
                let tid = match self.current_resume_tid {
                    // NOTE: Can't single-step all cores.
                    SpecificIdKind::All => return Err(Error::PacketUnexpected),
                    SpecificIdKind::WithId(tid) => tid,
                };

                let thread_id =
                    T::Tid::from_fully_qualified_tid(tid).ok_or(Error::UnexpectedThreadId)?;

                ops.reverse_step(thread_id).map_err(Error::TargetError)?;

                HandlerStatus::DoResume
            }
        };

        Ok(handler_status)
    }
}
