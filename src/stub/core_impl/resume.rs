use super::prelude::*;
use crate::arch::Arch;
use crate::common::Signal;
use crate::common::Tid;
use crate::protocol::commands::_vCont::Actions;
use crate::protocol::commands::ext::Resume;
use crate::protocol::SpecificIdKind;
use crate::protocol::SpecificThreadId;
use crate::target::ext::base::reverse_exec::ReplayLogPosition;
use crate::target::ext::base::ResumeOps;
use crate::target::ext::breakpoints::WatchKind;
use crate::target::ext::catch_syscalls::CatchSyscallPosition;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_stop_resume(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Resume<'_>,
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
            Resume::c(cmd) => {
                let _addr = cmd.addr;
                Actions::new_continue(SpecificThreadId {
                    pid: None,
                    tid: self.current_resume_tid,
                })
            }
            Resume::s(cmd) => {
                let _addr = cmd.addr;
                Actions::new_step(SpecificThreadId {
                    pid: None,
                    tid: self.current_resume_tid,
                })
            }
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

        // In the single-threaded scenario, we don't reverse the actions like we
        // do in the multi-threaded: there are only two scenarios we concern
        // ourselves with: 1 action, or 2 actions where the second is a
        // continue action (sometimes GDB sends a packet of the form
        // `vCont;s:foo;c`, even in single-threaded scenarios). We ignore the
        // continue action, since there aren't any other threads to continue.
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

        // NOTE: We iterate through these actions in reverse order, which corresponds to
        // a right-to-left ordering of the actions specified in the vCont packet. This
        // is intentionally the opposite of the left-to-right order specified by
        // the vCont packet documentation.
        //
        // This is to simplify target implementations: each `set_resume_action_XXX`
        // callback can overwrite the current state, instead of having to keep track of
        // each thread specified by previous actions and making sure they don't get
        // overwritten.
        for action in actions.iter().rev() {
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

                    let tid = match action.thread.map(|thread| thread.tid) {
                        // An action with no thread-id matches all threads, which is passed to
                        // `set_resume_action_continue` as `None`.
                        None | Some(SpecificIdKind::All) => None,
                        Some(SpecificIdKind::WithId(tid)) => Some(tid),
                    };

                    ops.set_resume_action_continue(tid, signal)
                        .map_err(Error::TargetError)?;
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

                // GDB doesn't always respect `vCont?` responses that omit `;s;S`, and will try to
                // send step packets regardless. Inform the user of this bug by issuing a
                // `UnexpectedStepPacket` error, which is more useful than a generic
                // `PacketUnexpected` error.
                VContKind::Step | VContKind::StepWithSig(..) => {
                    return Err(Error::UnexpectedStepPacket)
                }

                // Instead of using `_ =>`, explicitly list out any remaining unguarded cases.
                VContKind::RangeStep(..) => {
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

        Ok(HandlerStatus::DoResume)
    }

    fn write_stop_common(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Option<Tid>,
        signal: Signal,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("T")?;
        res.write_num(signal.0)?;

        if let Some(tid) = tid {
            self.current_mem_tid = tid;
            self.current_resume_tid = SpecificIdKind::WithId(tid);

            res.write_str("thread:")?;
            res.write_specific_thread_id(SpecificThreadId {
                pid: self
                    .features
                    .multiprocess()
                    .then_some(SpecificIdKind::WithId(self.get_current_pid(target)?)),
                tid: SpecificIdKind::WithId(tid),
            })?;
            res.write_str(";")?;
        }

        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_done_step(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("S")?;
        res.write_num(Signal::SIGTRAP.0)?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_signal(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        sig: Signal,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("S")?;
        res.write_num(sig.0)?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_exited(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        code: u8,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("W")?;
        res.write_num(code)?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_terminated(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        sig: Signal,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("X")?;
        res.write_num(sig.0)?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_signal_with_thread(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Tid,
        sig: Signal,
    ) -> Result<(), Error<T::Error, C::Error>> {
        self.write_stop_common(res, target, Some(tid), sig)?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_swbreak(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        if target
            .support_breakpoints()
            .and_then(|x| x.support_sw_breakpoint())
            .is_none()
        {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("sw_breakpoint", "stop_reason");

        self.write_stop_common(res, target, Some(tid), Signal::SIGTRAP)?;
        res.write_str("swbreak:;")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_hwbreak(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        if target
            .support_breakpoints()
            .and_then(|x| x.support_hw_breakpoint())
            .is_none()
        {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("hw_breakpoint", "stop_reason");

        self.write_stop_common(res, target, Some(tid), Signal::SIGTRAP)?;
        res.write_str("hwbreak:;")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_watch(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Tid,
        kind: WatchKind,
        addr: <<T as Target>::Arch as Arch>::Usize,
    ) -> Result<(), Error<T::Error, C::Error>> {
        if target
            .support_breakpoints()
            .and_then(|x| x.support_hw_watchpoint())
            .is_none()
        {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("hw_watchpoint", "stop_reason");

        self.write_stop_common(res, target, Some(tid), Signal::SIGTRAP)?;

        res.write_str(match kind {
            WatchKind::Write => "watch:",
            WatchKind::Read => "rwatch:",
            WatchKind::ReadWrite => "awatch:",
        })?;
        res.write_num(addr)?;
        res.write_str(";")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_replay_log(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Option<Tid>,
        pos: ReplayLogPosition,
    ) -> Result<(), Error<T::Error, C::Error>> {
        let supported = if let Some(resume_ops) = target.base_ops().resume_ops() {
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
        };

        if !supported {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("reverse_exec", "stop_reason");

        self.write_stop_common(res, target, tid, Signal::SIGTRAP)?;

        res.write_str("replaylog:")?;
        res.write_str(match pos {
            ReplayLogPosition::Begin => "begin",
            ReplayLogPosition::End => "end",
        })?;
        res.write_str(";")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_catch_syscall(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Option<Tid>,
        number: <<T as Target>::Arch as Arch>::Usize,
        position: CatchSyscallPosition,
    ) -> Result<(), Error<T::Error, C::Error>> {
        if target.support_catch_syscalls().is_none() {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("catch_syscall", "stop_reason");

        self.write_stop_common(res, target, tid, Signal::SIGTRAP)?;

        res.write_str(match position {
            CatchSyscallPosition::Entry => "syscall_entry:",
            CatchSyscallPosition::Return => "syscall_return:",
        })?;
        res.write_num(number)?;
        res.write_str(";")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_library(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        self.write_stop_common(res, target, Some(tid), Signal::SIGTRAP)?;
        res.write_str("library:;")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_fork(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        cur_tid: Tid,
        new_tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        if !target.use_fork_stop_reason() {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("fork_events", "stop_reason");
        self.write_stop_common(res, target, Some(cur_tid), Signal::SIGTRAP)?;
        res.write_str("fork:")?;
        res.write_specific_thread_id(SpecificThreadId {
            pid: self
                .features
                .multiprocess()
                .then_some(SpecificIdKind::WithId(self.get_current_pid(target)?)),
            tid: SpecificIdKind::WithId(new_tid),
        })?;
        res.write_str(";")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_vfork(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        cur_tid: Tid,
        new_tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        if !target.use_vfork_stop_reason() {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("vfork_events", "stop_reason");
        self.write_stop_common(res, target, Some(cur_tid), Signal::SIGTRAP)?;
        res.write_str("vfork:")?;
        res.write_specific_thread_id(SpecificThreadId {
            pid: self
                .features
                .multiprocess()
                .then_some(SpecificIdKind::WithId(self.get_current_pid(target)?)),
            tid: SpecificIdKind::WithId(new_tid),
        })?;
        res.write_str(";")?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn finish_vforkdone(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        if !target.use_vforkdone_stop_reason() {
            return Err(Error::UnsupportedStopReason);
        }

        crate::__dead_code_marker!("vforkdone_events", "stop_reason");
        self.write_stop_common(res, target, Some(tid), Signal::SIGTRAP)?;
        res.write_str("vforkdone:;")?;
        Ok(())
    }
}
