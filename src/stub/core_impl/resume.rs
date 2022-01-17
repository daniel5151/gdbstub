use super::prelude::*;
use crate::protocol::commands::ext::Resume;

use crate::arch::Arch;
use crate::common::Tid;
use crate::protocol::commands::_vCont::Actions;
use crate::protocol::{SpecificIdKind, SpecificThreadId};
use crate::stub::MultiThreadStopReason;
use crate::target::ext::base::reverse_exec::ReplayLogPosition;
use crate::target::ext::base::ResumeOps;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;
use crate::FAKE_PID;

use super::DisconnectReason;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_stop_resume<'a>(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Resume<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let mut ops = match target.base_ops().resume_ops() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let actions = match command {
            Resume::vCont(cmd) => {
                use crate::protocol::commands::_vCont::vCont;
                match cmd {
                    vCont::Query => {
                        // Continue is part of the base protocol
                        res.write_str("vCont;c;C")?;

                        // Single stepping is optional
                        if match &mut ops {
                            ResumeOps::SingleThread(ops) => ops.support_single_step().is_some(),
                            ResumeOps::MultiThread(ops) => ops.support_single_step().is_some(),
                        } {
                            res.write_str(";s;S")?;
                        }

                        // Range stepping is optional
                        if match &mut ops {
                            ResumeOps::SingleThread(ops) => ops.support_range_step().is_some(),
                            ResumeOps::MultiThread(ops) => ops.support_range_step().is_some(),
                        } {
                            res.write_str(";r")?;
                        }

                        // doesn't actually invoke vCont
                        return Ok(HandlerStatus::Handled);
                    }
                    vCont::Actions(actions) => actions,
                }
            }
            // TODO?: support custom resume addr in 'c' and 's'
            //
            // vCont doesn't have a notion of "resume addr", and since the implementation of these
            // packets reuse vCont infrastructure, supporting this obscure feature will be a bit
            // annoying...
            //
            // TODO: add `support_legacy_s_c_packets` flag (similar to `use_X_packet`)
            Resume::c(_) => Actions::new_continue(SpecificThreadId {
                pid: None,
                tid: self.current_resume_tid,
            }),
            Resume::s(_) => Actions::new_step(SpecificThreadId {
                pid: None,
                tid: self.current_resume_tid,
            }),
        };

        self.do_vcont(ops, actions)
    }

    fn do_vcont_single_thread(
        ops: &mut dyn crate::target::ext::base::singlethread::SingleThreadResume<
            Arch = T::Arch,
            Error = T::Error,
        >,
        actions: &Actions<'_>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        use crate::protocol::commands::_vCont::VContKind;

        let mut actions = actions.iter();
        let first_action = actions
            .next()
            .ok_or(Error::PacketParse(
                crate::protocol::PacketParseError::MalformedCommand,
            ))?
            .ok_or(Error::PacketParse(
                crate::protocol::PacketParseError::MalformedCommand,
            ))?;

        let invalid_second_action = match actions.next() {
            None => false,
            Some(act) => match act {
                None => {
                    return Err(Error::PacketParse(
                        crate::protocol::PacketParseError::MalformedCommand,
                    ))
                }
                Some(act) => !matches!(act.kind, VContKind::Continue),
            },
        };

        if invalid_second_action || actions.next().is_some() {
            return Err(Error::PacketUnexpected);
        }

        match first_action.kind {
            VContKind::Continue | VContKind::ContinueWithSig(_) => {
                let signal = match first_action.kind {
                    VContKind::ContinueWithSig(sig) => Some(sig),
                    _ => None,
                };

                ops.resume(signal).map_err(Error::TargetError)?;
                Ok(())
            }
            VContKind::Step | VContKind::StepWithSig(_) if ops.support_single_step().is_some() => {
                let ops = ops.support_single_step().unwrap();

                let signal = match first_action.kind {
                    VContKind::StepWithSig(sig) => Some(sig),
                    _ => None,
                };

                ops.step(signal).map_err(Error::TargetError)?;
                Ok(())
            }
            VContKind::RangeStep(start, end) if ops.support_range_step().is_some() => {
                let ops = ops.support_range_step().unwrap();

                let start = start.decode().map_err(|_| Error::TargetMismatch)?;
                let end = end.decode().map_err(|_| Error::TargetMismatch)?;

                ops.resume_range_step(start, end)
                    .map_err(Error::TargetError)?;
                Ok(())
            }
            // TODO: update this case when non-stop mode is implemented
            VContKind::Stop => Err(Error::PacketUnexpected),

            // Instead of using `_ =>`, explicitly list out any remaining unguarded cases.
            VContKind::RangeStep(..) | VContKind::Step | VContKind::StepWithSig(..) => {
                error!("GDB client sent resume action not reported by `vCont?`");
                Err(Error::PacketUnexpected)
            }
        }
    }

    fn do_vcont_multi_thread(
        ops: &mut dyn crate::target::ext::base::multithread::MultiThreadResume<
            Arch = T::Arch,
            Error = T::Error,
        >,
        actions: &Actions<'_>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        ops.clear_resume_actions().map_err(Error::TargetError)?;

        for action in actions.iter() {
            use crate::protocol::commands::_vCont::VContKind;

            let action = action.ok_or(Error::PacketParse(
                crate::protocol::PacketParseError::MalformedCommand,
            ))?;

            match action.kind {
                VContKind::Continue | VContKind::ContinueWithSig(_) => {
                    let signal = match action.kind {
                        VContKind::ContinueWithSig(sig) => Some(sig),
                        _ => None,
                    };

                    match action.thread.map(|thread| thread.tid) {
                        // An action with no thread-id matches all threads
                        None | Some(SpecificIdKind::All) => {
                            // Target API contract specifies that the default
                            // resume action for all threads is continue.
                        }
                        Some(SpecificIdKind::WithId(tid)) => ops
                            .set_resume_action_continue(tid, signal)
                            .map_err(Error::TargetError)?,
                    }
                }
                VContKind::Step | VContKind::StepWithSig(_)
                    if ops.support_single_step().is_some() =>
                {
                    let ops = ops.support_single_step().unwrap();

                    let signal = match action.kind {
                        VContKind::StepWithSig(sig) => Some(sig),
                        _ => None,
                    };

                    match action.thread.map(|thread| thread.tid) {
                        // An action with no thread-id matches all threads
                        None | Some(SpecificIdKind::All) => {
                            error!("GDB client sent 'step' as default resume action");
                            return Err(Error::PacketUnexpected);
                        }
                        Some(SpecificIdKind::WithId(tid)) => {
                            ops.set_resume_action_step(tid, signal)
                                .map_err(Error::TargetError)?;
                        }
                    };
                }

                VContKind::RangeStep(start, end) if ops.support_range_step().is_some() => {
                    let ops = ops.support_range_step().unwrap();

                    match action.thread.map(|thread| thread.tid) {
                        // An action with no thread-id matches all threads
                        None | Some(SpecificIdKind::All) => {
                            error!("GDB client sent 'range step' as default resume action");
                            return Err(Error::PacketUnexpected);
                        }
                        Some(SpecificIdKind::WithId(tid)) => {
                            let start = start.decode().map_err(|_| Error::TargetMismatch)?;
                            let end = end.decode().map_err(|_| Error::TargetMismatch)?;

                            ops.set_resume_action_range_step(tid, start, end)
                                .map_err(Error::TargetError)?;
                        }
                    };
                }
                // TODO: update this case when non-stop mode is implemented
                VContKind::Stop => return Err(Error::PacketUnexpected),

                // Instead of using `_ =>`, explicitly list out any remaining unguarded cases.
                VContKind::RangeStep(..) | VContKind::Step | VContKind::StepWithSig(..) => {
                    error!("GDB client sent resume action not reported by `vCont?`");
                    return Err(Error::PacketUnexpected);
                }
            }
        }

        ops.resume().map_err(Error::TargetError)
    }

    fn do_vcont(
        &mut self,
        ops: ResumeOps<'_, T::Arch, T::Error>,
        actions: Actions<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        match ops {
            ResumeOps::SingleThread(ops) => Self::do_vcont_single_thread(ops, &actions)?,
            ResumeOps::MultiThread(ops) => Self::do_vcont_multi_thread(ops, &actions)?,
        };

        Ok(HandlerStatus::DeferredStopReason)
    }

    fn write_break_common(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        self.current_mem_tid = tid;
        self.current_resume_tid = SpecificIdKind::WithId(tid);

        res.write_str("T05")?;

        res.write_str("thread:")?;
        res.write_specific_thread_id(SpecificThreadId {
            pid: self
                .features
                .multiprocess()
                .then(|| SpecificIdKind::WithId(FAKE_PID)),
            tid: SpecificIdKind::WithId(tid),
        })?;
        res.write_str(";")?;

        Ok(())
    }

    pub(crate) fn finish_exec(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        stop_reason: MultiThreadStopReason<<T::Arch as Arch>::Usize>,
    ) -> Result<FinishExecStatus, Error<T::Error, C::Error>> {
        macro_rules! guard_reverse_exec {
            () => {{
                if let Some(resume_ops) = target.base_ops().resume_ops() {
                    let (reverse_cont, reverse_step) = match resume_ops {
                        ResumeOps::MultiThread(ops) => (
                            ops.support_reverse_cont().is_some(),
                            ops.support_reverse_step().is_some(),
                        ),
                        ResumeOps::SingleThread(ops) => (
                            ops.support_reverse_cont().is_some(),
                            ops.support_reverse_step().is_some(),
                        ),
                    };

                    reverse_cont || reverse_step
                } else {
                    false
                }
            }};
        }

        macro_rules! guard_break {
            ($op:ident) => {
                target
                    .support_breakpoints()
                    .and_then(|ops| ops.$op())
                    .is_some()
            };
        }

        macro_rules! guard_catch_syscall {
            () => {
                target.support_catch_syscalls().is_some()
            };
        }

        let status = match stop_reason {
            MultiThreadStopReason::DoneStep => {
                res.write_str("S05")?;
                FinishExecStatus::Handled
            }
            MultiThreadStopReason::Signal(sig) => {
                res.write_str("S")?;
                res.write_num(sig as u8)?;
                FinishExecStatus::Handled
            }
            MultiThreadStopReason::Exited(code) => {
                res.write_str("W")?;
                res.write_num(code)?;
                FinishExecStatus::Disconnect(DisconnectReason::TargetExited(code))
            }
            MultiThreadStopReason::Terminated(sig) => {
                res.write_str("X")?;
                res.write_num(sig as u8)?;
                FinishExecStatus::Disconnect(DisconnectReason::TargetTerminated(sig))
            }
            MultiThreadStopReason::SwBreak(tid) if guard_break!(support_sw_breakpoint) => {
                crate::__dead_code_marker!("sw_breakpoint", "stop_reason");

                self.write_break_common(res, tid)?;
                res.write_str("swbreak:;")?;
                FinishExecStatus::Handled
            }
            MultiThreadStopReason::HwBreak(tid) if guard_break!(support_hw_breakpoint) => {
                crate::__dead_code_marker!("hw_breakpoint", "stop_reason");

                self.write_break_common(res, tid)?;
                res.write_str("hwbreak:;")?;
                FinishExecStatus::Handled
            }
            MultiThreadStopReason::Watch { tid, kind, addr }
                if guard_break!(support_hw_watchpoint) =>
            {
                crate::__dead_code_marker!("hw_watchpoint", "stop_reason");

                self.write_break_common(res, tid)?;

                use crate::target::ext::breakpoints::WatchKind;
                match kind {
                    WatchKind::Write => res.write_str("watch:")?,
                    WatchKind::Read => res.write_str("rwatch:")?,
                    WatchKind::ReadWrite => res.write_str("awatch:")?,
                }
                res.write_num(addr)?;
                res.write_str(";")?;
                FinishExecStatus::Handled
            }
            MultiThreadStopReason::ReplayLog(pos) if guard_reverse_exec!() => {
                crate::__dead_code_marker!("reverse_exec", "stop_reason");

                res.write_str("T05")?;

                res.write_str("replaylog:")?;
                res.write_str(match pos {
                    ReplayLogPosition::Begin => "begin",
                    ReplayLogPosition::End => "end",
                })?;
                res.write_str(";")?;

                FinishExecStatus::Handled
            }
            MultiThreadStopReason::CatchSyscall { number, position } if guard_catch_syscall!() => {
                crate::__dead_code_marker!("catch_syscall", "stop_reason");

                res.write_str("T05")?;

                res.write_str(match position {
                    CatchSyscallPosition::Entry => "syscall_entry:",
                    CatchSyscallPosition::Return => "syscall_return:",
                })?;
                res.write_num(number)?;
                res.write_str(";")?;

                FinishExecStatus::Handled
            }
            // Explicitly avoid using `_ =>` to handle the "unguarded" variants, as doing so would
            // squelch the useful compiler error that crops up whenever stop reasons are added.
            MultiThreadStopReason::SwBreak(_)
            | MultiThreadStopReason::HwBreak(_)
            | MultiThreadStopReason::Watch { .. }
            | MultiThreadStopReason::ReplayLog(_)
            | MultiThreadStopReason::CatchSyscall { .. } => {
                return Err(Error::UnsupportedStopReason);
            }
        };

        Ok(status)
    }
}

pub(crate) enum FinishExecStatus {
    Handled,
    Disconnect(DisconnectReason),
}
