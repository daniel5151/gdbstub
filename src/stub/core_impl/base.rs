use super::prelude::*;
use crate::protocol::commands::ext::Base;

use crate::arch::{Arch, Registers};
use crate::common::Tid;
use crate::protocol::{IdKind, SpecificIdKind, SpecificThreadId};
use crate::target::ext::base::{BaseOps, ResumeOps};
use crate::{FAKE_PID, SINGLE_THREAD_TID};

use super::DisconnectReason;

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
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Base<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let handler_status = match command {
            // ------------------ Handshaking and Queries ------------------- //
            Base::qSupported(cmd) => {
                use crate::protocol::commands::_qSupported::Feature;

                // perform incoming feature negotiation
                for feature in cmd.features.into_iter() {
                    let (feature, supported) = match feature {
                        Ok(Some(v)) => v,
                        Ok(None) => continue,
                        Err(()) => {
                            return Err(Error::PacketParse(
                                crate::protocol::PacketParseError::MalformedCommand,
                            ))
                        }
                    };

                    match feature {
                        Feature::Multiprocess => self.features.set_multiprocess(supported),
                    }
                }

                res.write_str("PacketSize=")?;
                res.write_num(cmd.packet_buffer_len)?;

                // these are the few features that gdbstub unconditionally supports
                res.write_str(concat!(
                    ";vContSupported+",
                    ";multiprocess+",
                    ";QStartNoAckMode+",
                ))?;

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

                    if reverse_cont {
                        res.write_str(";ReverseContinue+")?;
                    }

                    if reverse_step {
                        res.write_str(";ReverseStep+")?;
                    }
                }

                if let Some(ops) = target.support_extended_mode() {
                    if ops.support_configure_aslr().is_some() {
                        res.write_str(";QDisableRandomization+")?;
                    }

                    if ops.support_configure_env().is_some() {
                        res.write_str(";QEnvironmentHexEncoded+")?;
                        res.write_str(";QEnvironmentUnset+")?;
                        res.write_str(";QEnvironmentReset+")?;
                    }

                    if ops.support_configure_startup_shell().is_some() {
                        res.write_str(";QStartupWithShell+")?;
                    }

                    if ops.support_configure_working_dir().is_some() {
                        res.write_str(";QSetWorkingDir+")?;
                    }
                }

                if let Some(ops) = target.support_breakpoints() {
                    if ops.support_sw_breakpoint().is_some() {
                        res.write_str(";swbreak+")?;
                    }

                    if ops.support_hw_breakpoint().is_some()
                        || ops.support_hw_watchpoint().is_some()
                    {
                        res.write_str(";hwbreak+")?;
                    }
                }

                if target.support_catch_syscalls().is_some() {
                    res.write_str(";QCatchSyscalls+")?;
                }

                if target.use_target_description_xml()
                    && (T::Arch::target_description_xml().is_some()
                        || target.support_target_description_xml_override().is_some())
                {
                    res.write_str(";qXfer:features:read+")?;
                }

                if target.support_memory_map().is_some() {
                    res.write_str(";qXfer:memory-map:read+")?;
                }

                if target.support_exec_file().is_some() {
                    res.write_str(";qXfer:exec-file:read+")?;
                }

                if target.support_auxv().is_some() {
                    res.write_str(";qXfer:auxv:read+")?;
                }

                HandlerStatus::Handled
            }
            Base::QStartNoAckMode(_) => {
                self.features.set_no_ack_mode(true);
                HandlerStatus::NeedsOk
            }

            // -------------------- "Core" Functionality -------------------- //
            // TODO: Improve the '?' response based on last-sent stop reason.
            // this will be particularly relevant when working on non-stop mode.
            Base::QuestionMark(_) => {
                // Reply with a valid thread-id or GDB issues a warning when more
                // than one thread is active
                res.write_str("T05thread:")?;
                res.write_specific_thread_id(SpecificThreadId {
                    pid: self
                        .features
                        .multiprocess()
                        .then(|| SpecificIdKind::WithId(FAKE_PID)),
                    tid: SpecificIdKind::WithId(self.get_sane_any_tid(target)?),
                })?;
                res.write_str(";")?;
                HandlerStatus::Handled
            }
            Base::qAttached(cmd) => {
                let is_attached = match target.support_extended_mode() {
                    // when _not_ running in extended mode, just report that we're attaching to an
                    // existing process.
                    None => true, // assume attached to an existing process
                    // When running in extended mode, we must defer to the target
                    Some(ops) => {
                        match cmd.pid {
                            Some(pid) => ops.query_if_attached(pid).handle_error()?.was_attached(),
                            None => true, // assume attached to an existing process
                        }
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
                match target.support_extended_mode() {
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
                        pid: self
                            .features
                            .multiprocess()
                            .then(|| SpecificIdKind::WithId(FAKE_PID)),
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
                                    pid: self
                                        .features
                                        .multiprocess()
                                        .then(|| SpecificIdKind::WithId(FAKE_PID)),
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
}
