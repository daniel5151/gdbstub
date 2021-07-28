use super::prelude::*;
use crate::protocol::commands::ext::Base;

use crate::arch::{Arch, Registers};
use crate::protocol::{IdKind, SpecificIdKind, SpecificThreadId};
use crate::target::ext::base::multithread::ThreadStopReason;
use crate::target::ext::base::{BaseOps, GdbInterrupt, ReplayLogPosition, ResumeAction};
use crate::{FAKE_PID, SINGLE_THREAD_TID};

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    #[inline(always)]
    fn get_sane_any_tid(&mut self, target: &mut T) -> Result<Tid, Error<T::Error, C::Error>> {
        let tid = match target.base_ops() {
            BaseOps::SingleThread(_) => SINGLE_THREAD_TID,
            BaseOps::MultiThread(ops) => {
                let mut first_tid = None;
                ops.list_active_threads(&mut |tid| {
                    if first_tid.is_none() {
                        first_tid = Some(tid);
                    }
                })
                .map_err(Error::TargetError)?;
                // Note that `Error::NoActiveThreads` shouldn't ever occur, since this method is
                // called from the `H` packet handler, which AFAIK is only sent after the GDB
                // client has confirmed that a thread / process exists.
                //
                // If it does, that really sucks, and will require rethinking how to handle "any
                // thread" messages.
                first_tid.ok_or(Error::NoActiveThreads)?
            }
        };
        Ok(tid)
    }

    pub(crate) fn handle_base<'a>(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: Base<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let handler_status = match command {
            // ------------------ Handshaking and Queries ------------------- //
            Base::qSupported(cmd) => {
                // XXX: actually read what the client supports, and enable/disable features
                // appropriately
                let _features = cmd.features.into_iter();

                res.write_str("PacketSize=")?;
                res.write_num(cmd.packet_buffer_len)?;

                res.write_str(";vContSupported+")?;
                res.write_str(";multiprocess+")?;
                res.write_str(";QStartNoAckMode+")?;

                let (reverse_cont, reverse_step) = match target.base_ops() {
                    BaseOps::MultiThread(ops) => (
                        ops.support_reverse_cont().is_some(),
                        ops.support_reverse_step().is_some(),
                    ),
                    BaseOps::SingleThread(ops) => (
                        ops.support_reverse_cont().is_some(),
                        ops.support_reverse_step().is_some(),
                    ),
                };

                if reverse_cont {
                    res.write_str(";ReverseContinue+")?;
                }

                if reverse_step {
                    res.write_str(";ReverseStep+")?;
                }

                if let Some(ops) = target.extended_mode() {
                    if ops.configure_aslr().is_some() {
                        res.write_str(";QDisableRandomization+")?;
                    }

                    if ops.configure_env().is_some() {
                        res.write_str(";QEnvironmentHexEncoded+")?;
                        res.write_str(";QEnvironmentUnset+")?;
                        res.write_str(";QEnvironmentReset+")?;
                    }

                    if ops.configure_startup_shell().is_some() {
                        res.write_str(";QStartupWithShell+")?;
                    }

                    if ops.configure_working_dir().is_some() {
                        res.write_str(";QSetWorkingDir+")?;
                    }
                }

                if let Some(ops) = target.breakpoints() {
                    if ops.sw_breakpoint().is_some() {
                        res.write_str(";swbreak+")?;
                    }

                    if ops.hw_breakpoint().is_some() || ops.hw_watchpoint().is_some() {
                        res.write_str(";hwbreak+")?;
                    }
                }

                if target.catch_syscalls().is_some() {
                    res.write_str(";QCatchSyscalls+")?;
                }

                if T::Arch::target_description_xml().is_some()
                    || target.target_description_xml_override().is_some()
                {
                    res.write_str(";qXfer:features:read+")?;
                }

                if target.memory_map().is_some() {
                    res.write_str(";qXfer:memory-map:read+")?;
                }

                HandlerStatus::Handled
            }
            Base::QStartNoAckMode(_) => {
                self.no_ack_mode = true;
                HandlerStatus::NeedsOk
            }
            Base::qXferFeaturesRead(cmd) => {
                #[allow(clippy::redundant_closure)]
                let xml = target
                    .target_description_xml_override()
                    .map(|ops| ops.target_description_xml())
                    .or_else(|| T::Arch::target_description_xml());

                match xml {
                    Some(xml) => {
                        let xml = xml.trim().as_bytes();
                        res.write_binary_range(xml, cmd.offset, cmd.len)?;
                    }
                    // If the target hasn't provided their own XML, then the initial response to
                    // "qSupported" wouldn't have included "qXfer:features:read", and gdb wouldn't
                    // send this packet unless it was explicitly marked as supported.
                    None => return Err(Error::PacketUnexpected),
                }
                HandlerStatus::Handled
            }

            // -------------------- "Core" Functionality -------------------- //
            // TODO: Improve the '?' response based on last-sent stop reason.
            // this will be particularly relevant when working on non-stop mode.
            Base::QuestionMark(_) => {
                res.write_str("S05")?;
                HandlerStatus::Handled
            }
            Base::qAttached(cmd) => {
                let is_attached = match target.extended_mode() {
                    // when _not_ running in extended mode, just report that we're attaching to an
                    // existing process.
                    None => true, // assume attached to an existing process
                    // When running in extended mode, we must defer to the target
                    Some(ops) => {
                        let pid: Pid = cmd.pid.ok_or(Error::PacketUnexpected)?;
                        ops.query_if_attached(pid).handle_error()?.was_attached()
                    }
                };
                res.write_str(if is_attached { "1" } else { "0" })?;
                HandlerStatus::Handled
            }
            Base::g(_) => {
                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.read_registers(&mut regs),
                    BaseOps::MultiThread(ops) => {
                        ops.read_registers(&mut regs, self.current_mem_tid)
                    }
                }
                .handle_error()?;

                let mut err = Ok(());
                regs.gdb_serialize(|val| {
                    let res = match val {
                        Some(b) => res.write_hex_buf(&[b]),
                        None => res.write_str("xx"),
                    };
                    if let Err(e) = res {
                        err = Err(e);
                    }
                });
                err?;
                HandlerStatus::Handled
            }
            Base::G(cmd) => {
                let mut regs: <T::Arch as Arch>::Registers = Default::default();
                regs.gdb_deserialize(cmd.vals)
                    .map_err(|_| Error::TargetMismatch)?;

                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.write_registers(&regs),
                    BaseOps::MultiThread(ops) => ops.write_registers(&regs, self.current_mem_tid),
                }
                .handle_error()?;

                HandlerStatus::NeedsOk
            }
            Base::m(cmd) => {
                let buf = cmd.buf;
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                let mut i = 0;
                let mut n = cmd.len;
                while n != 0 {
                    let chunk_size = n.min(buf.len());

                    use num_traits::NumCast;

                    let addr = addr + NumCast::from(i).ok_or(Error::TargetMismatch)?;
                    let data = &mut buf[..chunk_size];
                    match target.base_ops() {
                        BaseOps::SingleThread(ops) => ops.read_addrs(addr, data),
                        BaseOps::MultiThread(ops) => {
                            ops.read_addrs(addr, data, self.current_mem_tid)
                        }
                    }
                    .handle_error()?;

                    n -= chunk_size;
                    i += chunk_size;

                    res.write_hex_buf(data)?;
                }
                HandlerStatus::Handled
            }
            Base::M(cmd) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(cmd.addr)
                    .ok_or(Error::TargetMismatch)?;

                match target.base_ops() {
                    BaseOps::SingleThread(ops) => ops.write_addrs(addr, cmd.val),
                    BaseOps::MultiThread(ops) => {
                        ops.write_addrs(addr, cmd.val, self.current_mem_tid)
                    }
                }
                .handle_error()?;

                HandlerStatus::NeedsOk
            }
            Base::k(_) | Base::vKill(_) => {
                match target.extended_mode() {
                    // When not running in extended mode, stop the `GdbStub` and disconnect.
                    None => HandlerStatus::Disconnect(DisconnectReason::Kill),

                    // When running in extended mode, a kill command does not necessarily result in
                    // a disconnect...
                    Some(ops) => {
                        let pid = match command {
                            Base::vKill(cmd) => Some(cmd.pid),
                            _ => None,
                        };

                        let should_terminate = ops.kill(pid).handle_error()?;
                        if should_terminate.into_bool() {
                            // manually write OK, since we need to return a DisconnectReason
                            res.write_str("OK")?;
                            HandlerStatus::Disconnect(DisconnectReason::Kill)
                        } else {
                            HandlerStatus::NeedsOk
                        }
                    }
                }
            }
            Base::D(_) => {
                // TODO: plumb-through Pid when exposing full multiprocess + extended mode
                res.write_str("OK")?; // manually write OK, since we need to return a DisconnectReason
                HandlerStatus::Disconnect(DisconnectReason::Disconnect)
            }
            Base::vCont(cmd) => {
                use crate::protocol::commands::_vCont::vCont;
                match cmd {
                    vCont::Query => {
                        res.write_str("vCont;c;C;s;S")?;
                        if match target.base_ops() {
                            BaseOps::SingleThread(ops) => ops.support_resume_range_step().is_some(),
                            BaseOps::MultiThread(ops) => ops.support_range_step().is_some(),
                        } {
                            res.write_str(";r")?;
                        }
                        HandlerStatus::Handled
                    }
                    vCont::Actions(actions) => self.do_vcont(res, target, actions)?,
                }
            }
            // TODO?: support custom resume addr in 'c' and 's'
            //
            // unfortunately, this wouldn't be a particularly easy thing to implement, since the
            // vCont packet doesn't natively support custom resume addresses. This leaves a few
            // options for the implementation:
            //
            // 1. Adding new ResumeActions (i.e: ContinueWithAddr(U) and StepWithAddr(U))
            // 2. Automatically calling `read_registers`, updating the `pc`, and calling
            //    `write_registers` prior to resuming.
            //    - will require adding some sort of `get_pc_mut` method to the `Registers` trait.
            //
            // Option 1 is easier to implement, but puts more burden on the implementor. Option 2
            // will require more effort to implement (and will be less performant), but it will hide
            // this protocol wart from the end user.
            //
            // Oh, one more thought - there's a subtle pitfall to watch out for if implementing
            // Option 1: if the target is using conditional breakpoints, `do_vcont` has to be
            // modified to only pass the resume with address variants on the _first_ iteration
            // through the loop.
            Base::c(_) => {
                use crate::protocol::commands::_vCont::Actions;

                self.do_vcont(
                    res,
                    target,
                    Actions::new_continue(SpecificThreadId {
                        pid: None,
                        tid: self.current_resume_tid,
                    }),
                )?
            }
            Base::s(_) => {
                use crate::protocol::commands::_vCont::Actions;

                self.do_vcont(
                    res,
                    target,
                    Actions::new_step(SpecificThreadId {
                        pid: None,
                        tid: self.current_resume_tid,
                    }),
                )?
            }

            // ------------------- Multi-threading Support ------------------ //
            Base::H(cmd) => {
                use crate::protocol::commands::_h_upcase::Op;
                match cmd.kind {
                    Op::Other => match cmd.thread.tid {
                        IdKind::Any => self.current_mem_tid = self.get_sane_any_tid(target)?,
                        // "All" threads doesn't make sense for memory accesses
                        IdKind::All => return Err(Error::PacketUnexpected),
                        IdKind::WithId(tid) => self.current_mem_tid = tid,
                    },
                    // technically, this variant is deprecated in favor of vCont...
                    Op::StepContinue => match cmd.thread.tid {
                        IdKind::Any => {
                            self.current_resume_tid =
                                SpecificIdKind::WithId(self.get_sane_any_tid(target)?)
                        }
                        IdKind::All => self.current_resume_tid = SpecificIdKind::All,
                        IdKind::WithId(tid) => {
                            self.current_resume_tid = SpecificIdKind::WithId(tid)
                        }
                    },
                }
                HandlerStatus::NeedsOk
            }
            Base::qfThreadInfo(_) => {
                res.write_str("m")?;

                match target.base_ops() {
                    BaseOps::SingleThread(_) => res.write_specific_thread_id(SpecificThreadId {
                        pid: Some(SpecificIdKind::WithId(FAKE_PID)),
                        tid: SpecificIdKind::WithId(SINGLE_THREAD_TID),
                    })?,
                    BaseOps::MultiThread(ops) => {
                        let mut err: Result<_, Error<T::Error, C::Error>> = Ok(());
                        let mut first = true;
                        ops.list_active_threads(&mut |tid| {
                            // TODO: replace this with a try block (once stabilized)
                            let e = (|| {
                                if !first {
                                    res.write_str(",")?
                                }
                                first = false;
                                res.write_specific_thread_id(SpecificThreadId {
                                    pid: Some(SpecificIdKind::WithId(FAKE_PID)),
                                    tid: SpecificIdKind::WithId(tid),
                                })?;
                                Ok(())
                            })();

                            if let Err(e) = e {
                                err = Err(e)
                            }
                        })
                        .map_err(Error::TargetError)?;
                        err?;
                    }
                }

                HandlerStatus::Handled
            }
            Base::qsThreadInfo(_) => {
                res.write_str("l")?;
                HandlerStatus::Handled
            }
            Base::T(cmd) => {
                let alive = match cmd.thread.tid {
                    IdKind::WithId(tid) => match target.base_ops() {
                        BaseOps::SingleThread(_) => tid == SINGLE_THREAD_TID,
                        BaseOps::MultiThread(ops) => {
                            ops.is_thread_alive(tid).map_err(Error::TargetError)?
                        }
                    },
                    // TODO: double-check if GDB ever sends other variants
                    // Even after ample testing, this arm has never been hit...
                    _ => return Err(Error::PacketUnexpected),
                };
                if alive {
                    HandlerStatus::NeedsOk
                } else {
                    // any error code will do
                    return Err(Error::NonFatalError(1));
                }
            }
        };
        Ok(handler_status)
    }

    fn do_vcont_single_thread(
        ops: &mut dyn crate::target::ext::base::singlethread::SingleThreadOps<
            Arch = T::Arch,
            Error = T::Error,
        >,
        res: &mut ResponseWriter<C>,
        actions: &crate::protocol::commands::_vCont::Actions,
    ) -> Result<Option<ThreadStopReason<<T::Arch as Arch>::Usize>>, Error<T::Error, C::Error>> {
        use crate::protocol::commands::_vCont::VContKind;

        let mut err = Ok(());
        let mut check_gdb_interrupt = || match res.as_conn().peek() {
            Ok(Some(0x03)) => true, // 0x03 is the interrupt byte
            Ok(Some(_)) => false,   // it's nothing that can't wait...
            Ok(None) => false,
            Err(e) => {
                err = Err(Error::ConnectionRead(e));
                true // break ASAP if a connection error occurred
            }
        };

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

        let action = match first_action.kind {
            VContKind::Step => ResumeAction::Step,
            VContKind::Continue => ResumeAction::Continue,
            VContKind::StepWithSig(sig) => ResumeAction::StepWithSignal(sig),
            VContKind::ContinueWithSig(sig) => ResumeAction::ContinueWithSignal(sig),
            VContKind::RangeStep(start, end) => {
                if let Some(ops) = ops.support_resume_range_step() {
                    let start = start.decode().map_err(|_| Error::TargetMismatch)?;
                    let end = end.decode().map_err(|_| Error::TargetMismatch)?;

                    let ret = ops
                        .resume_range_step(start, end, GdbInterrupt::new(&mut check_gdb_interrupt))
                        .map_err(Error::TargetError)?
                        .map(Into::into);
                    err?;
                    return Ok(ret);
                } else {
                    return Err(Error::PacketUnexpected);
                }
            }
            // TODO: update this case when non-stop mode is implemented
            VContKind::Stop => return Err(Error::PacketUnexpected),
        };

        let ret = ops
            .resume(action, GdbInterrupt::new(&mut check_gdb_interrupt))
            .map_err(Error::TargetError)?
            .map(Into::into);
        err?;
        Ok(ret)
    }

    fn do_vcont_multi_thread(
        ops: &mut dyn crate::target::ext::base::multithread::MultiThreadOps<
            Arch = T::Arch,
            Error = T::Error,
        >,
        res: &mut ResponseWriter<C>,
        actions: &crate::protocol::commands::_vCont::Actions,
    ) -> Result<Option<ThreadStopReason<<T::Arch as Arch>::Usize>>, Error<T::Error, C::Error>> {
        // this is a pretty arbitrary choice, but it seems reasonable for most cases.
        let mut default_resume_action = ResumeAction::Continue;

        ops.clear_resume_actions().map_err(Error::TargetError)?;

        for action in actions.iter() {
            use crate::protocol::commands::_vCont::VContKind;

            let action = action.ok_or(Error::PacketParse(
                crate::protocol::PacketParseError::MalformedCommand,
            ))?;

            let resume_action = match action.kind {
                VContKind::Step => ResumeAction::Step,
                VContKind::Continue => ResumeAction::Continue,
                // there seems to be a GDB bug where it doesn't use `vCont` unless
                // `vCont?` returns support for resuming with a signal.
                VContKind::StepWithSig(sig) => ResumeAction::StepWithSignal(sig),
                VContKind::ContinueWithSig(sig) => ResumeAction::ContinueWithSignal(sig),
                VContKind::RangeStep(start, end) => {
                    if let Some(ops) = ops.support_range_step() {
                        match action.thread.map(|thread| thread.tid) {
                            // An action with no thread-id matches all threads
                            None | Some(SpecificIdKind::All) => {
                                return Err(Error::PacketUnexpected)
                            }
                            Some(SpecificIdKind::WithId(tid)) => {
                                let start = start.decode().map_err(|_| Error::TargetMismatch)?;
                                let end = end.decode().map_err(|_| Error::TargetMismatch)?;

                                ops.set_resume_action_range_step(tid, start, end)
                                    .map_err(Error::TargetError)?;
                                continue;
                            }
                        };
                    } else {
                        return Err(Error::PacketUnexpected);
                    }
                }
                // TODO: update this case when non-stop mode is implemented
                VContKind::Stop => return Err(Error::PacketUnexpected),
            };

            match action.thread.map(|thread| thread.tid) {
                // An action with no thread-id matches all threads
                None | Some(SpecificIdKind::All) => default_resume_action = resume_action,
                Some(SpecificIdKind::WithId(tid)) => ops
                    .set_resume_action(tid, resume_action)
                    .map_err(Error::TargetError)?,
            };
        }

        let mut err = Ok(());
        let mut check_gdb_interrupt = || match res.as_conn().peek() {
            Ok(Some(0x03)) => true, // 0x03 is the interrupt byte
            Ok(Some(_)) => false,   // it's nothing that can't wait...
            Ok(None) => false,
            Err(e) => {
                err = Err(Error::ConnectionRead(e));
                true // break ASAP if a connection error occurred
            }
        };

        let ret = ops
            .resume(
                default_resume_action,
                GdbInterrupt::new(&mut check_gdb_interrupt),
            )
            .map_err(Error::TargetError)?;

        err?;

        Ok(ret)
    }

    fn do_vcont(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        actions: crate::protocol::commands::_vCont::Actions,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let stop_reason = match target.base_ops() {
            BaseOps::SingleThread(ops) => Self::do_vcont_single_thread(ops, res, &actions)?,
            BaseOps::MultiThread(ops) => Self::do_vcont_multi_thread(ops, res, &actions)?,
        };

        match stop_reason {
            None => Ok(HandlerStatus::DeferredStopReason),
            Some(stop_reason) => self
                .finish_exec(res, target, stop_reason)
                .map(|x| x.into_handler_status()),
        }
    }

    fn write_break_common(
        &mut self,
        res: &mut ResponseWriter<C>,
        tid: Tid,
    ) -> Result<(), Error<T::Error, C::Error>> {
        self.current_mem_tid = tid;
        self.current_resume_tid = SpecificIdKind::WithId(tid);

        res.write_str("T05")?;

        res.write_str("thread:")?;
        res.write_specific_thread_id(SpecificThreadId {
            pid: Some(SpecificIdKind::WithId(FAKE_PID)),
            tid: SpecificIdKind::WithId(tid),
        })?;
        res.write_str(";")?;

        Ok(())
    }

    pub(crate) fn finish_exec(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        stop_reason: ThreadStopReason<<T::Arch as Arch>::Usize>,
    ) -> Result<FinishExecStatus, Error<T::Error, C::Error>> {
        macro_rules! guard_reverse_exec {
            () => {{
                let (reverse_cont, reverse_step) = match target.base_ops() {
                    BaseOps::MultiThread(ops) => (
                        ops.support_reverse_cont().is_some(),
                        ops.support_reverse_step().is_some(),
                    ),
                    BaseOps::SingleThread(ops) => (
                        ops.support_reverse_cont().is_some(),
                        ops.support_reverse_step().is_some(),
                    ),
                };
                reverse_cont || reverse_step
            }};
        }

        macro_rules! guard_break {
            ($op:ident) => {
                target.breakpoints().and_then(|ops| ops.$op()).is_some()
            };
        }

        macro_rules! guard_catch_syscall {
            () => {
                target.catch_syscalls().is_some()
            };
        }

        let status = match stop_reason {
            ThreadStopReason::DoneStep | ThreadStopReason::GdbInterrupt => {
                res.write_str("S05")?;
                FinishExecStatus::Handled
            }
            ThreadStopReason::Signal(sig) => {
                res.write_str("S")?;
                res.write_num(sig)?;
                FinishExecStatus::Handled
            }
            ThreadStopReason::Exited(code) => {
                res.write_str("W")?;
                res.write_num(code)?;
                FinishExecStatus::Disconnect(DisconnectReason::TargetExited(code))
            }
            ThreadStopReason::Terminated(sig) => {
                res.write_str("X")?;
                res.write_num(sig)?;
                FinishExecStatus::Disconnect(DisconnectReason::TargetTerminated(sig))
            }
            ThreadStopReason::SwBreak(tid) if guard_break!(sw_breakpoint) => {
                crate::__dead_code_marker!("sw_breakpoint", "stop_reason");

                self.write_break_common(res, tid)?;
                res.write_str("swbreak:;")?;
                FinishExecStatus::Handled
            }
            ThreadStopReason::HwBreak(tid) if guard_break!(hw_breakpoint) => {
                crate::__dead_code_marker!("hw_breakpoint", "stop_reason");

                self.write_break_common(res, tid)?;
                res.write_str("hwbreak:;")?;
                FinishExecStatus::Handled
            }
            ThreadStopReason::Watch { tid, kind, addr } if guard_break!(hw_watchpoint) => {
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
            ThreadStopReason::ReplayLog(pos) if guard_reverse_exec!() => {
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
            // Explicitly avoid using `_ =>` to handle the "unguarded" variants, as doing so would
            // squelch the useful compiler error that crops up whenever stop reasons are added.
            ThreadStopReason::SwBreak(_)
            | ThreadStopReason::HwBreak(_)
            | ThreadStopReason::Watch { .. }
            | ThreadStopReason::ReplayLog(_) => {
                return Err(Error::UnsupportedStopReason);
            }
            ThreadStopReason::CatchSyscall { number, position } if guard_catch_syscall!() => {
                crate::__dead_code_marker!("catch_syscall", "stop_reason");

                res.write_str("T05")?;

                use crate::target::ext::catch_syscalls::CatchSyscallPosition;
                res.write_str(match position {
                    CatchSyscallPosition::Entry => "syscall_entry:",
                    CatchSyscallPosition::Return => "syscall_return:",
                })?;
                res.write_num(number)?;
                res.write_str(";")?;

                FinishExecStatus::Handled
            }
            _ => return Err(Error::UnsupportedStopReason),
        };

        Ok(status)
    }
}

pub(crate) enum FinishExecStatus {
    Handled,
    Disconnect(DisconnectReason),
}

impl FinishExecStatus {
    pub(super) fn into_handler_status(self) -> HandlerStatus {
        match self {
            FinishExecStatus::Disconnect(status) => HandlerStatus::Disconnect(status),
            FinishExecStatus::Handled => HandlerStatus::Handled,
        }
    }
}

use crate::target::ext::base::singlethread::StopReason;
impl<U> From<StopReason<U>> for ThreadStopReason<U> {
    fn from(st_stop_reason: StopReason<U>) -> ThreadStopReason<U> {
        match st_stop_reason {
            StopReason::DoneStep => ThreadStopReason::DoneStep,
            StopReason::GdbInterrupt => ThreadStopReason::GdbInterrupt,
            StopReason::Exited(code) => ThreadStopReason::Exited(code),
            StopReason::Terminated(sig) => ThreadStopReason::Terminated(sig),
            StopReason::SwBreak => ThreadStopReason::SwBreak(SINGLE_THREAD_TID),
            StopReason::HwBreak => ThreadStopReason::HwBreak(SINGLE_THREAD_TID),
            StopReason::Watch { kind, addr } => ThreadStopReason::Watch {
                tid: SINGLE_THREAD_TID,
                kind,
                addr,
            },
            StopReason::Signal(sig) => ThreadStopReason::Signal(sig),
            StopReason::ReplayLog(pos) => ThreadStopReason::ReplayLog(pos),
            StopReason::CatchSyscall { number, position } => {
                ThreadStopReason::CatchSyscall { number, position }
            }
        }
    }
}
