use super::prelude::*;
use crate::protocol::commands::ext::{ReverseCont, ReverseStep};

use crate::arch::Arch;
use crate::protocol::SpecificIdKind;
use crate::target::ext::base::multithread::{MultiThreadReverseCont, MultiThreadReverseStep};
use crate::target::ext::base::singlethread::{SingleThreadReverseCont, SingleThreadReverseStep};
use crate::target::ext::base::BaseOps;

enum ReverseContOps<'a, A: Arch, E> {
    SingleThread(&'a mut dyn SingleThreadReverseCont<Arch = A, Error = E>),
    MultiThread(&'a mut dyn MultiThreadReverseCont<Arch = A, Error = E>),
}

enum ReverseStepOps<'a, A: Arch, E> {
    SingleThread(&'a mut dyn SingleThreadReverseStep<Arch = A, Error = E>),
    MultiThread(&'a mut dyn MultiThreadReverseStep<Arch = A, Error = E>),
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_reverse_cont(
        &mut self,
        _res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ReverseCont,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.base_ops() {
            BaseOps::MultiThread(ops) => match ops.support_reverse_cont() {
                Some(ops) => ReverseContOps::MultiThread(ops),
                None => return Ok(HandlerStatus::Handled),
            },
            BaseOps::SingleThread(ops) => match ops.support_reverse_cont() {
                Some(ops) => ReverseContOps::SingleThread(ops),
                None => return Ok(HandlerStatus::Handled),
            },
        };

        crate::__dead_code_marker!("reverse_cont", "impl");

        let handler_status = match command {
            ReverseCont::bc(_) => {
                match ops {
                    ReverseContOps::MultiThread(ops) => {
                        ops.reverse_cont().map_err(Error::TargetError)?
                    }
                    ReverseContOps::SingleThread(ops) => {
                        ops.reverse_cont().map_err(Error::TargetError)?
                    }
                }

                HandlerStatus::DeferredStopReason
            }
        };

        Ok(handler_status)
    }

    // FIXME: De-duplicate with above code?
    pub(crate) fn handle_reverse_step(
        &mut self,
        _res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ReverseStep,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.base_ops() {
            BaseOps::MultiThread(ops) => match ops.support_reverse_step() {
                Some(ops) => ReverseStepOps::MultiThread(ops),
                None => return Ok(HandlerStatus::Handled),
            },
            BaseOps::SingleThread(ops) => match ops.support_reverse_step() {
                Some(ops) => ReverseStepOps::SingleThread(ops),
                None => return Ok(HandlerStatus::Handled),
            },
        };

        crate::__dead_code_marker!("reverse_step", "impl");

        let handler_status = match command {
            ReverseStep::bs(_) => {
                let tid = match self.current_resume_tid {
                    // NOTE: Can't single-step all cores.
                    SpecificIdKind::All => return Err(Error::PacketUnexpected),
                    SpecificIdKind::WithId(tid) => tid,
                };

                match ops {
                    ReverseStepOps::MultiThread(ops) => {
                        ops.reverse_step(tid).map_err(Error::TargetError)?
                    }
                    ReverseStepOps::SingleThread(ops) => {
                        ops.reverse_step().map_err(Error::TargetError)?
                    }
                }

                HandlerStatus::DeferredStopReason
            }
        };

        Ok(handler_status)
    }
}
