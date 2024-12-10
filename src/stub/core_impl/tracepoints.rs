use super::prelude::*;
use crate::arch::Arch;
use crate::internal::BeBytes;
use crate::protocol::commands::_QTBuffer::QTBuffer;
use crate::protocol::commands::ext::Tracepoints;
use crate::protocol::commands::prelude::decode_hex;
use crate::protocol::commands::prelude::decode_hex_buf;
use crate::protocol::commands::_QTDP::CreateTDP;
use crate::protocol::commands::_QTDP::DefineTDP;
use crate::protocol::commands::_QTDP::QTDP;
use crate::target::ext::tracepoints::DefineTracepoint;
use crate::target::ext::tracepoints::FrameDescription;
use crate::target::ext::tracepoints::FrameRequest;
use crate::target::ext::tracepoints::NewTracepoint;
use crate::target::ext::tracepoints::TracepointAction;
use crate::target::ext::tracepoints::TracepointActionList;
use crate::target::ext::tracepoints::TracepointItem;
use managed::ManagedSlice;

impl<U: BeBytes> NewTracepoint<U> {
    /// Parse from a raw CreateTDP packet.
    pub fn from_tdp(ctdp: CreateTDP<'_>) -> Option<Self> {
        let arch_int = |b: &[u8]| -> Result<U, ()> { U::from_be_bytes(b).ok_or(()) };
        Some(Self {
            number: ctdp.number,
            addr: arch_int(ctdp.addr).ok()?,
            enabled: ctdp.enable,
            pass_count: ctdp.pass,
            step_count: ctdp.step,
            more: ctdp.more,
        })
    }
}

impl<'a, U: BeBytes> DefineTracepoint<'a, U> {
    /// Parse from a raw DefineTDP packet.
    pub fn from_tdp(dtdp: DefineTDP<'a>) -> Option<Self> {
        let arch_int = |b: &[u8]| -> Result<U, ()> { U::from_be_bytes(b).ok_or(()) };

        Some(Self {
            number: dtdp.number,
            addr: arch_int(dtdp.addr).ok()?,
            actions: TracepointActionList::Raw { data: ManagedSlice::Borrowed(dtdp.actions) },
        })
    }

    /// Parse the actions that should be added to the definition of this
    /// tracepoint, calling `f` on each action.
    ///
    /// Returns None if parsing of actions failed, or hit unsupported actions.
    /// Return `Some(more)` on success, where `more` is a bool indicating if
    /// there will be additional "tracepoint define" packets for this
    /// tracepoint.
    pub fn actions(self, mut f: impl FnMut(&TracepointAction<'_, U>)) -> Option<bool> {
        match self.actions {
            TracepointActionList::Raw { mut data } => Self::parse_raw_actions(&mut data, f),
            TracepointActionList::Parsed { mut actions, more } => {
                for action in actions.iter_mut() {
                    (f)(action);
                }
                Some(more)
            }
        }
    }

    fn parse_raw_actions(
        actions: &mut [u8],
        mut f: impl FnMut(&TracepointAction<'_, U>),
    ) -> Option<bool> {
        let mut more = false;
        let mut unparsed: Option<&mut [u8]> = Some(actions);
        loop {
            match unparsed {
                Some([b'S', ..]) => {
                    // TODO: how can gdbstub even implement this? it changes how
                    // future packets should be interpreted, but as a trait we
                    // can't keep a flag around for that (unless we specifically
                    // have a `mark_while_stepping` callback for the target to
                    // keep track future tracepoint_defines should be treated different).
                    // If we go that route we also would need to return two vectors
                    // here, "normal" actions and "while stepping" actions...but
                    // "normals" actions may still be "while stepping" actions,
                    // just continued from the previous packet, which we forgot
                    // about!
                    return None;
                }
                Some([b'R', mask @ ..]) => {
                    let mask_end = mask
                        .iter()
                        .cloned()
                        .enumerate()
                        .find(|(_i, b)| matches!(b, b'S' | b'R' | b'M' | b'X' | b'-'));
                    // We may or may not have another action after our mask
                    let mask = if let Some(mask_end) = mask_end {
                        let (mask_bytes, next) = mask.split_at_mut(mask_end.0);
                        unparsed = Some(next);
                        decode_hex_buf(mask_bytes).ok()?
                    } else {
                        unparsed = None;
                        decode_hex_buf(mask).ok()?
                    };
                    (f)(&TracepointAction::Registers { mask: ManagedSlice::Borrowed(mask) });
                }
                Some([b'M', _mem_args @ ..]) => {
                    // Unimplemented: even simple actions like `collect *(int*)0x0`
                    // are actually assembled as `X` bytecode actions
                    return None;
                }
                Some([b'X', eval_args @ ..]) => {
                    let mut len_end = eval_args.splitn_mut(2, |b| *b == b',');
                    let (len_bytes, rem) = (len_end.next()?, len_end.next()?);
                    let len: usize = decode_hex(len_bytes).ok()?;
                    let (expr_bytes, next_bytes) = rem.split_at_mut(len * 2);
                    let expr = decode_hex_buf(expr_bytes).ok()?;
                    (f)(&TracepointAction::Expression { expr: ManagedSlice::Borrowed(expr) });
                    unparsed = Some(next_bytes);
                }
                Some([b'-']) => {
                    more = true;
                    break;
                }
                Some([]) | None => {
                    break;
                }
                _ => return None,
            }
        }

        Some(more)
    }
}

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_tracepoints(
        &mut self,
        res: &mut ResponseWriter<'_, C>,
        target: &mut T,
        command: Tracepoints<'_>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.support_tracepoints() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        crate::__dead_code_marker!("tracepoints", "impl");

        match command {
            Tracepoints::QTinit(_) => {
                ops.tracepoints_init().handle_error()?;
                // GDB documentation doesn't say it, but this requires "OK" in order
                // to signify we support tracepoints.
                return Ok(HandlerStatus::NeedsOk);
            }
            Tracepoints::qTStatus(_) => {
                let status = ops.trace_experiment_status().handle_error()?;
                res.write_str("T")?;
                res.write_dec(if status.running { 1 } else { 0 })?;
                for explanation in status.explanations.iter() {
                    res.write_str(";")?;
                    explanation.write(res)?;
                }
            }
            Tracepoints::qTP(qtp) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(qtp.addr)
                    .ok_or(Error::TargetMismatch)?;
                let (hits, usage) = ops.tracepoint_status(qtp.tracepoint, addr).handle_error()?;
                res.write_str("V")?;
                res.write_num(hits)?;
                res.write_str(":")?;
                res.write_num(usage)?;
            }
            Tracepoints::QTDP(q) => {
                match q {
                    QTDP::Create(ctdp) => {
                        let new_tracepoint =
                            NewTracepoint::<<T::Arch as Arch>::Usize>::from_tdp(ctdp)
                                .ok_or(Error::TargetMismatch)?;
                        ops.tracepoint_create(new_tracepoint).handle_error()?
                    }
                    QTDP::Define(dtdp) => {
                        let define_tracepoint =
                            DefineTracepoint::<<T::Arch as Arch>::Usize>::from_tdp(dtdp)
                                .ok_or(Error::TargetMismatch)?;
                        ops.tracepoint_define(define_tracepoint).handle_error()?
                    }
                };
                // TODO: support qRelocInsn?
                return Ok(HandlerStatus::NeedsOk);
            }
            Tracepoints::QTBuffer(buf) => {
                match buf {
                    QTBuffer::Request {
                        offset,
                        length,
                        data,
                    } => {
                        let read = ops
                            .trace_buffer_request(offset, length, data)
                            .handle_error()?;
                        if let Some(read) = read {
                            let read = read.min(data.len());
                            res.write_hex_buf(&data[..read])?;
                        } else {
                            res.write_str("l")?;
                        }
                    }
                    QTBuffer::Configure { buffer } => {
                        ops.trace_buffer_configure(buffer).handle_error()?;
                    }
                }
                // Documentation doesn't mention this, but it needs OK
                return Ok(HandlerStatus::NeedsOk);
            }
            Tracepoints::QTStart(_) => {
                ops.trace_experiment_start().handle_error()?;
                // Documentation doesn't mention this, but it needs OK
                // TODO: qRelocInsn reply support?
                return Ok(HandlerStatus::NeedsOk);
            }
            Tracepoints::QTStop(_) => {
                ops.trace_experiment_stop().handle_error()?;
                // Documentation doesn't mention this, but it needs OK
                return Ok(HandlerStatus::NeedsOk);
            }
            Tracepoints::QTFrame(req) => {
                let parsed_qtframe: Option<FrameRequest<<T::Arch as Arch>::Usize>> = req.0.into();
                let parsed_req = parsed_qtframe.ok_or(Error::TargetMismatch)?;
                let mut err: Result<_, Error<T::Error, C::Error>> = Ok(());
                let mut any_results = false;
                ops.select_frame(parsed_req, &mut |desc| {
                    // TODO: bubble up the C::Error from this closure? list_thread_active does this
                    // instead
                    let e = (|| -> Result<_, _> {
                        match desc {
                            FrameDescription::FrameNumber(n) => {
                                res.write_str("F ")?;
                                if let Some(n) = n {
                                    res.write_num(n)?;
                                } else {
                                    res.write_str("-1")?;
                                }
                            }
                            FrameDescription::Hit(tdp) => {
                                res.write_str("T ")?;
                                res.write_num(tdp.0)?;
                            }
                        }
                        any_results = true;
                        Ok(())
                    })();
                    if let Err(e) = e {
                        err = Err(e)
                    }
                })
                .handle_error()?;
                if !any_results {
                    res.write_str("F -1")?;
                }
            }
            // The GDB protocol for this is very weird: it sends this first packet
            // to initialize a state machine on our stub, and then sends the subsequent
            // packets N times in order to drive the state machine to the end in
            // order to ask for all our tracepoints and their actions. gdbstub
            // is agnostic about the end-user state and shouldn't allocate, however,
            // which means we can't really setup the state machine ourselves or
            // turn it into a nicer API.
            Tracepoints::qTfP(_) => {
                let tp = ops.tracepoint_enumerate_start().handle_error()?;
                if let Some(tp) = tp {
                    match tp {
                        TracepointItem::New(ctp) => ctp.write(res)?,
                        TracepointItem::Define(dtp) => dtp.write(res)?,
                    }
                }
            }
            Tracepoints::qTsP(_) => {
                let tp = ops.tracepoint_enumerate_step().handle_error()?;
                if let Some(tp) = tp {
                    match tp {
                        TracepointItem::New(ctp) => ctp.write(res)?,
                        TracepointItem::Define(dtp) => dtp.write(res)?,
                    }
                }
            }

            // Likewise, the same type of driven state machine is used for trace
            // state variables
            Tracepoints::qTfV(_) => {
                // TODO: State variables
            }
            Tracepoints::qTsV(_) => {
                // TODO: State variables
            }
        };

        Ok(HandlerStatus::Handled)
    }
}
