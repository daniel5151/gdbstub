use super::prelude::*;
use crate::protocol::commands::ext::{ReverseCont, ReverseStep};

use crate::arch::Arch;
use crate::common::Tid;
use crate::protocol::SpecificIdKind;
use crate::target::ext::base::reverse_exec::{
    ReverseCont as ReverseContTrait, ReverseStep as ReverseStepTrait,
};
use crate::target::ext::base::ResumeOps;

macro_rules! defn_ops {
    ($name:ident, $reverse_trait:ident, $f:ident) => {
        enum $name<'a, A: Arch, E> {
            SingleThread(&'a mut dyn $reverse_trait<(), Arch = A, Error = E>),
            MultiThread(&'a mut dyn $reverse_trait<Tid, Arch = A, Error = E>),
        }

        impl<'a, A, E> $name<'a, A, E>
        where
            A: Arch,
        {
            #[inline(always)]
            fn from_target<T>(target: &mut T) -> Option<$name<'_, T::Arch, T::Error>>
            where
                T: Target,
            {
                let ops = match target.base_ops().resume_ops()? {
                    ResumeOps::SingleThread(ops) => $name::SingleThread(ops.$f()?),
                    ResumeOps::MultiThread(ops) => $name::MultiThread(ops.$f()?),
                };
                Some(ops)
            }
        }
    };
}

defn_ops!(ReverseContOps, ReverseContTrait, support_reverse_cont);
defn_ops!(ReverseStepOps, ReverseStepTrait, support_reverse_step);

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_reverse_cont(
        &mut self,
        _res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: ReverseCont,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match ReverseContOps::<'_, T::Arch, T::Error>::from_target(target) {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
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
        _res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: ReverseStep,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match ReverseStepOps::<'_, T::Arch, T::Error>::from_target(target) {
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

                match ops {
                    ReverseStepOps::MultiThread(ops) => {
                        ops.reverse_step(tid).map_err(Error::TargetError)?
                    }
                    ReverseStepOps::SingleThread(ops) => {
                        ops.reverse_step(()).map_err(Error::TargetError)?
                    }
                }

                HandlerStatus::DeferredStopReason
            }
        };

        Ok(handler_status)
    }
}
