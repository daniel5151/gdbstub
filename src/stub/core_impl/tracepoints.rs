use super::prelude::*;
use crate::arch::Arch;
use crate::internal::BeBytes;
use crate::protocol::commands::_QTDPsrc::QTDPsrc;
use crate::protocol::commands::_qTBuffer::qTBuffer;
use crate::protocol::commands::ext::Tracepoints;
use crate::protocol::commands::prelude::decode_hex;
use crate::protocol::commands::prelude::decode_hex_buf;
use crate::protocol::commands::_QTDP::CreateTDP;
use crate::protocol::commands::_QTDP::ExtendTDP;
use crate::protocol::commands::_QTDP::QTDP;
use crate::protocol::ResponseWriterError;
use crate::target::ext::tracepoints::ExperimentExplanation;
use crate::target::ext::tracepoints::ExperimentStatus;
use crate::target::ext::tracepoints::FrameDescription;
use crate::target::ext::tracepoints::FrameRequest;
use crate::target::ext::tracepoints::NewTracepoint;
use crate::target::ext::tracepoints::SourceTracepoint;
use crate::target::ext::tracepoints::Tracepoint;
use crate::target::ext::tracepoints::TracepointAction;
use crate::target::ext::tracepoints::TracepointEnumerateCursor;
use crate::target::ext::tracepoints::TracepointEnumerateStep;
use crate::target::ext::tracepoints::TracepointSourceType;
use crate::target::ext::tracepoints::TracepointStatus;
use managed::ManagedSlice;
use num_traits::PrimInt;

impl<U: BeBytes> NewTracepoint<U> {
    /// Parse from a raw CreateTDP packet.
    fn from_tdp(ctdp: CreateTDP<'_>) -> Option<(Self, bool)> {
        Some((
            Self {
                number: ctdp.number,
                addr: U::from_be_bytes(ctdp.addr)?,
                enabled: ctdp.enable,
                pass_count: ctdp.pass,
                step_count: ctdp.step,
            },
            ctdp.more,
        ))
    }
}

impl<U: crate::internal::BeBytes + num_traits::Zero + PrimInt> NewTracepoint<U> {
    /// Write this as a qTfP/qTsP response
    pub(crate) fn write<T: Target, C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("T")?;
        res.write_num(self.number.0)?;
        res.write_str(":")?;
        res.write_num(self.addr)?;
        res.write_str(":")?;
        res.write_str(if self.enabled { "E" } else { "D" })?;
        res.write_str(":")?;
        res.write_num(self.step_count)?;
        res.write_str(":")?;
        res.write_num(self.pass_count)?;

        Ok(())
    }
}

/// A list of actions that a tracepoint should be extended with.
#[derive(Debug)]
pub(crate) struct ExtendTracepoint<'a, U> {
    /// The tracepoint that is having actions appended to its definition.
    pub number: Tracepoint,
    /// The PC address of the tracepoint that is being extended.
    /// This is currently unused information sent as part of the packet by GDB,
    /// but may be required for implementing while-stepping actions later.
    #[allow(dead_code)]
    pub addr: U,
    /// The unparsed action data.
    pub data: ManagedSlice<'a, u8>,
}

impl<'a, U: BeBytes> ExtendTracepoint<'a, U> {
    /// Parse from a raw ExtendTDP packet.
    fn from_tdp(dtdp: ExtendTDP<'a>) -> Option<Self> {
        Some(Self {
            number: dtdp.number,
            addr: U::from_be_bytes(dtdp.addr)?,
            data: ManagedSlice::Borrowed(dtdp.actions),
        })
    }

    /// Parse the actions that should be added to the definition of this
    /// tracepoint, calling `f` on each action.
    ///
    /// Returns `Err` if parsing of actions failed, or hit unsupported actions.
    /// Return `Ok(more)` on success, where more indicates if more actions are
    /// expect in later packets. If the actions weren't from a GDB packet, more
    /// is None.
    pub(crate) fn actions<T, C>(
        mut self,
        f: impl FnMut(&TracepointAction<'_, U>),
    ) -> Result<Option<bool>, Error<T, C>> {
        Self::parse_raw_actions(&mut self.data, f)
    }

    fn parse_raw_actions<T, C>(
        actions: &mut [u8],
        mut f: impl FnMut(&TracepointAction<'_, U>),
    ) -> Result<Option<bool>, Error<T, C>> {
        let (actions, more) = match actions {
            [rest @ .., b'-'] => (rest, true),
            x => (x, false),
        };
        // TODO: There's no "packet unsupported", so for now we stub out unimplemented
        // functionality by reporting the commands malformed instead.
        use crate::protocol::PacketParseError::MalformedCommand;
        let mut unparsed: Option<&mut [u8]> = Some(actions);
        loop {
            match unparsed {
                Some([b'S', ..]) => {
                    // TODO: how can gdbstub even implement this? it changes how
                    // future packets should be interpreted, but as a trait we
                    // can't keep a flag around for that (unless we specifically
                    // have a `mark_while_stepping` callback for the target to
                    // keep track future tracepoint_extends should be treated different).
                    // If we go that route we also would need to return two vectors
                    // here, "normal" actions and "while stepping" actions...but
                    // "normals" actions may still be "while stepping" actions,
                    // just continued from the previous packet, which we forgot
                    // about!
                    //
                    // We use 'W' to indicate "while-stepping", since we're already
                    // using 'S' elsewhere for static tracepoints.
                    return Err(Error::TracepointFeatureUnimplemented(b'W'));
                }
                Some([b'R', mask @ ..]) => {
                    let mask_end = mask
                        .iter()
                        .enumerate()
                        .find(|(_i, b)| matches!(b, b'S' | b'R' | b'M' | b'X'));
                    // We may or may not have another action after our mask
                    let mask = if let Some(mask_end) = mask_end {
                        let (mask_bytes, next) = mask.split_at_mut(mask_end.0);
                        unparsed = Some(next);
                        decode_hex_buf(mask_bytes).or(Err(Error::PacketParse(MalformedCommand)))?
                    } else {
                        unparsed = None;
                        decode_hex_buf(mask).or(Err(Error::PacketParse(MalformedCommand)))?
                    };
                    (f)(&TracepointAction::Registers {
                        mask: ManagedSlice::Borrowed(mask),
                    });
                }
                Some([b'M', _mem_args @ ..]) => {
                    // Unimplemented: even simple actions like `collect *(int*)0x0`
                    // are actually assembled as `X` bytecode actions
                    return Err(Error::TracepointFeatureUnimplemented(b'M'));
                }
                Some([b'X', eval_args @ ..]) => {
                    let mut len_end = eval_args.splitn_mut(2, |b| *b == b',');
                    let (len_bytes, rem) = (
                        len_end.next().ok_or(Error::PacketParse(MalformedCommand))?,
                        len_end.next().ok_or(Error::PacketParse(MalformedCommand))?,
                    );
                    let len: usize =
                        decode_hex(len_bytes).or(Err(Error::PacketParse(MalformedCommand)))?;
                    if rem.len() < len * 2 {
                        return Err(Error::PacketParse(MalformedCommand));
                    }
                    let (expr_bytes, next_bytes) = rem.split_at_mut(len * 2);
                    let expr =
                        decode_hex_buf(expr_bytes).or(Err(Error::PacketParse(MalformedCommand)))?;
                    (f)(&TracepointAction::Expression {
                        expr: ManagedSlice::Borrowed(expr),
                    });
                    unparsed = Some(next_bytes);
                }
                Some([]) | None => {
                    break;
                }
                _ => return Err(Error::PacketParse(MalformedCommand)),
            }
        }

        Ok(Some(more))
    }
}

impl<'a, U: BeBytes> SourceTracepoint<'a, U> {
    /// Parse from a raw CreateTDP packet.
    fn from_src(src: QTDPsrc<'a>) -> Option<Self> {
        Some(Self {
            number: src.number,
            addr: U::from_be_bytes(src.addr)?,
            kind: src.kind,
            start: src.start,
            slen: src.slen,
            bytes: ManagedSlice::Borrowed(src.bytes),
        })
    }
}
impl<U: crate::internal::BeBytes + num_traits::Zero + PrimInt> SourceTracepoint<'_, U> {
    /// Write this as a qTfP/qTsP response
    pub(crate) fn write<T: Target, C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("Z")?;
        res.write_num(self.number.0)?;
        res.write_str(":")?;
        res.write_num(self.addr)?;
        res.write_str(":")?;
        res.write_str(match self.kind {
            TracepointSourceType::At => "at",
            TracepointSourceType::Cond => "cond",
            TracepointSourceType::Cmd => "cmd",
        })?;
        res.write_str(":")?;
        // We use the start and slen from the SourceTracepoint instead of
        // start=0 slen=bytes.len() because, although we can assume GDB to be able
        // to handle arbitrary sized packets, the target implementation might still
        // be giving us chunked source (such as if it's parroting the chunked source
        // that GDB initially gave us).
        res.write_num(self.start)?;
        res.write_str(":")?;
        res.write_num(self.slen)?;
        res.write_str(":")?;
        res.write_hex_buf(self.bytes.as_ref())?;

        Ok(())
    }
}

impl<'a, U: crate::internal::BeBytes + num_traits::Zero + PrimInt> TracepointAction<'a, U> {
    /// Write this as a qTfP/qTsP response
    pub(crate) fn write<T: Target, C: Connection>(
        &self,
        tp: Tracepoint,
        addr: U,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        res.write_str("A")?;
        res.write_num(tp.0)?;
        res.write_str(":")?;
        res.write_num(addr)?;
        res.write_str(":")?;

        match self {
            TracepointAction::Registers { mask } => {
                res.write_str("R")?;
                res.write_hex_buf(mask)?;
            }
            TracepointAction::Memory {
                basereg,
                offset,
                length,
            } => {
                res.write_str("M")?;
                match basereg {
                    Some(r) => res.write_num(*r),
                    None => res.write_str("-1"),
                }?;
                res.write_str(",")?;
                res.write_num(*offset)?;
                res.write_str(",")?;
                res.write_num(*length)?;
            }
            TracepointAction::Expression { expr } => {
                res.write_str("X")?;
                res.write_num(expr.len())?;
                res.write_str(",")?;
                res.write_hex_buf(expr)?;
            }
        }
        Ok(())
    }
}

impl<'a> ExperimentStatus<'a> {
    pub(crate) fn write<C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), ResponseWriterError<C::Error>> {
        use crate::target::ext::tracepoints::ExperimentStatus::*;
        if let Running = self {
            return res.write_str("T1");
        }
        // We're stopped for some reason, and may have an explanation for why
        res.write_str("T0")?;
        match self {
            Running => { /* unreachable */ }
            NotRunning => { /* no information */ }
            NotRun => res.write_str(";tnotrun:0")?,
            Stop(ref t) => match t {
                Some(text) => {
                    res.write_str(";tstop:")?;
                    res.write_hex_buf(text)?;
                    res.write_str(":0")?;
                }
                None => res.write_str(";tstop:0")?,
            },
            Full => res.write_str(";tfull:0")?,
            Disconnected => res.write_str(";tdisconnected:0")?,
            PassCount(tpnum) => {
                res.write_str(";tpasscount:")?;
                res.write_num(tpnum.0)?;
            }
            Error(text, tpnum) => {
                res.write_str(";terror:")?;
                res.write_hex_buf(text)?;
                res.write_str(":")?;
                res.write_num(tpnum.0)?;
            }
            Unknown => res.write_str(";tunknown:0")?,
        }

        Ok(())
    }
}

impl<'a> ExperimentExplanation<'a> {
    pub(crate) fn write<T: Target, C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        use ExperimentExplanation::*;
        match self {
            Frames(u) => {
                res.write_str("tframes:")?;
                res.write_num(*u)?;
            }
            Created(u) => {
                res.write_str("tcreated:")?;
                res.write_num(*u)?;
            }
            Size(u) => {
                res.write_str("tsize:")?;
                res.write_num(*u)?;
            }
            Free(u) => {
                res.write_str("tfree:")?;
                res.write_num(*u)?;
            }
            Circular(u) => {
                res.write_str("circular:")?;
                res.write_num(if *u { 1 } else { 0 })?;
            }
            Disconn(dis) => match dis {
                true => res.write_str("disconn:1")?,
                false => res.write_str("disconn:0")?,
            },
            Other(body) => res.write_str(body.as_ref())?,
        };

        Ok(())
    }
}

impl<'a, U: crate::internal::BeBytes> From<FrameRequest<&'a mut [u8]>> for Option<FrameRequest<U>> {
    fn from(s: FrameRequest<&'a mut [u8]>) -> Self {
        Some(match s {
            FrameRequest::Select(u) => FrameRequest::Select(u),
            FrameRequest::AtPC(u) => FrameRequest::AtPC(U::from_be_bytes(u)?),
            FrameRequest::Hit(tp) => FrameRequest::Hit(tp),
            FrameRequest::Between(s, e) => {
                FrameRequest::Between(U::from_be_bytes(s)?, U::from_be_bytes(e)?)
            }
            FrameRequest::Outside(s, e) => {
                FrameRequest::Outside(U::from_be_bytes(s)?, U::from_be_bytes(e)?)
            }
        })
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
                let mut err: Option<Error<T::Error, C::Error>> = None;
                let mut has_status = false;
                ops.trace_experiment_status(&mut |status: ExperimentStatus<'_>| {
                    // If the target implementation calls us multiple times, then
                    // we would erroneously serialize an invalid response. Guard
                    // against it in the simplest way.
                    if has_status {
                        return;
                    }
                    if let Err(e) = status.write(res) {
                        err = Some(e.into())
                    } else {
                        has_status = true;
                    }
                })
                .handle_error()?;
                if has_status {
                    // Only bother trying to get info if we also have a status
                    ops.trace_experiment_info(&mut |explanation: ExperimentExplanation<'_>| {
                        if let Err(e) = res
                            .write_str(";")
                            .map_err(|e| e.into())
                            .and_then(|()| explanation.write::<T, C>(res))
                        {
                            err = Some(e)
                        }
                    })
                    .handle_error()?;
                }

                if let Some(e) = err {
                    return Err(e);
                }
            }
            Tracepoints::qTP(qtp) => {
                let addr = <T::Arch as Arch>::Usize::from_be_bytes(qtp.addr)
                    .ok_or(Error::TargetMismatch)?;
                let TracepointStatus {
                    hit_count,
                    bytes_used,
                } = ops.tracepoint_status(qtp.tracepoint, addr).handle_error()?;
                res.write_str("V")?;
                res.write_num(hit_count)?;
                res.write_str(":")?;
                res.write_num(bytes_used)?;
            }
            Tracepoints::QTDP(q) => {
                match q {
                    QTDP::Create(ctdp) => {
                        if let Some(feat) = ctdp.unsupported_option {
                            // We have some options we don't know how to process, so bail out.
                            return Err(Error::TracepointFeatureUnimplemented(feat));
                        }

                        let (new_tracepoint, more) =
                            NewTracepoint::<<T::Arch as Arch>::Usize>::from_tdp(ctdp)
                                .ok_or(Error::TargetMismatch)?;
                        let tp = new_tracepoint.number;
                        ops.tracepoint_create_begin(new_tracepoint).handle_error()?;
                        if !more {
                            ops.tracepoint_create_complete(tp).handle_error()?;
                        }
                    }
                    QTDP::Extend(dtdp) => {
                        let extend_tracepoint =
                            ExtendTracepoint::<<T::Arch as Arch>::Usize>::from_tdp(dtdp)
                                .ok_or(Error::TargetMismatch)?;
                        let tp = extend_tracepoint.number;
                        let mut err: Option<Error<T::Error, C::Error>> = None;
                        let more = extend_tracepoint.actions(|action| {
                            if let Err(e) =
                                ops.tracepoint_create_continue(tp, action).handle_error()
                            {
                                err = Some(e)
                            }
                        });
                        if let Some(e) = err {
                            return Err(e);
                        }
                        match more {
                            Ok(Some(true)) => {
                                // We expect additional QTDP packets, so don't
                                // complete it yet.
                            }
                            Ok(None) | Ok(Some(false)) => {
                                ops.tracepoint_create_complete(tp).handle_error()?;
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                };
                // TODO: support qRelocInsn?
                return Ok(HandlerStatus::NeedsOk);
            }
            Tracepoints::QTDPsrc(src) => {
                if let Some(supports_sources) = ops.support_tracepoint_source() {
                    let source = SourceTracepoint::<<T::Arch as Arch>::Usize>::from_src(src)
                        .ok_or(Error::TargetMismatch)?;
                    supports_sources
                        .tracepoint_attach_source(source)
                        .handle_error()?;
                    // Documentation doesn't mention this, but it needs OK
                    return Ok(HandlerStatus::NeedsOk);
                }
            }
            Tracepoints::qTBuffer(buf) => {
                let qTBuffer { offset, length } = buf;
                let mut wrote: Result<bool, Error<T::Error, C::Error>> = Ok(false);
                ops.trace_buffer_request(offset, length, &mut |data| {
                    if let Err(e) = res.write_hex_buf(data) {
                        wrote = Err(e.into())
                    } else {
                        wrote = Ok(true)
                    }
                })
                .handle_error()?;
                if !wrote? {
                    res.write_str("l")?;
                }
            }
            Tracepoints::QTBuffer(conf) => {
                ops.trace_buffer_configure(conf.0).handle_error()?;
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
                    let e = (|| -> Result<_, _> {
                        match desc {
                            FrameDescription::FrameNumber(n) => {
                                res.write_str("F")?;
                                res.write_num(n)?;
                                any_results = true;
                            }
                            FrameDescription::Hit(tdp) => {
                                res.write_str("T")?;
                                res.write_num(tdp.0)?;
                            }
                        }
                        Ok(())
                    })();
                    if let Err(e) = e {
                        err = Err(e)
                    }
                })
                .handle_error()?;
                if !any_results {
                    res.write_str("F-1")?;
                }
            }
            // The GDB protocol for this is very weird: it sends this first packet
            // to initialize a state machine on our stub, and then sends the subsequent
            // packets N times in order to drive the state machine to the end in
            // order to ask for all our tracepoints and their actions. gdbstub
            // uses a target allocated state machine that it drives in response
            // to these packets, so that it can provide a nice typed API.
            Tracepoints::qTfP(_) => {
                // Reset our state machine
                let state = ops.tracepoint_enumerate_state();
                state.cursor = None;

                let mut err: Option<Error<T::Error, C::Error>> = None;
                let mut started = None;
                let step = ops
                    .tracepoint_enumerate_start(None, &mut |ctp| {
                        // We need to know what tracepoint to begin stepping, since the
                        // target will just tell us there's TracepointEnumerateStep::More
                        // otherwise.
                        started = Some((ctp.number, ctp.addr));
                        let e = ctp.write::<T, C>(res);
                        if let Err(e) = e {
                            err = Some(e)
                        }
                    })
                    .handle_error()?;
                if let Some(e) = err {
                    return Err(e);
                }
                if let Some((tp, addr)) = started {
                    ops.tracepoint_enumerate_state().cursor =
                        Some(TracepointEnumerateCursor::New { tp, addr });
                }
                self.handle_tracepoint_state_machine_step(target, step)?;
            }
            Tracepoints::qTsP(_) => {
                let state = ops.tracepoint_enumerate_state();
                let mut err: Option<Error<T::Error, C::Error>> = None;
                let step = match state.cursor {
                    None => {
                        // If we don't have a cursor, than the last
                        // packet responded
                        // with a TracepointEnumerateStep::Done. We don't have
                        // anything else to report.
                        None
                    }
                    Some(TracepointEnumerateCursor::New { tp, .. }) => {
                        // If we don't have any progress, the last packet was
                        // a Next(tp) and we need to start reporting a new tracepoint
                        Some(
                            ops.tracepoint_enumerate_start(Some(tp), &mut |ctp| {
                                let e = ctp.write::<T, C>(res);
                                if let Err(e) = e {
                                    err = Some(e)
                                }
                            })
                            .handle_error()?,
                        )
                    }
                    Some(TracepointEnumerateCursor::Action { tp, addr, step }) => {
                        // Otherwise we should be continuing the advance the current tracepoint.
                        Some(
                            ops.tracepoint_enumerate_action(tp, step, &mut |action| {
                                let e = action.write::<T, C>(tp, addr, res);
                                if let Err(e) = e {
                                    err = Some(e)
                                }
                            })
                            .handle_error()?,
                        )
                    }
                    Some(TracepointEnumerateCursor::Source { tp, step, .. }) => {
                        if let Some(supports_sources) = ops.support_tracepoint_source() {
                            Some(
                                supports_sources
                                    .tracepoint_enumerate_source(tp, step, &mut |src| {
                                        let e = src.write::<T, C>(res);
                                        if let Err(e) = e {
                                            err = Some(e)
                                        }
                                    })
                                    .handle_error()?,
                            )
                        } else {
                            // If the target doesn't support tracepoint sources but told
                            // us to enumerate one anyways, then all we can do is
                            // stop our state machine.
                            None
                        }
                    }
                };

                if let Some(e) = err {
                    return Err(e);
                }
                if let Some(step) = step {
                    self.handle_tracepoint_state_machine_step(target, step)?;
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

    fn handle_tracepoint_state_machine_step(
        &mut self,
        target: &mut T,
        step: TracepointEnumerateStep<<T::Arch as Arch>::Usize>,
    ) -> Result<(), Error<T::Error, C::Error>> {
        let ops = match target.support_tracepoints() {
            Some(ops) => ops,
            None => return Ok(()),
        };
        let state = ops.tracepoint_enumerate_state();
        let next = match (state.cursor.as_ref(), step) {
            (None, _) => None,
            (Some(_), TracepointEnumerateStep::Done) => None,

            // Transition to enumerating actions
            (
                Some(&TracepointEnumerateCursor::New { tp, addr }),
                TracepointEnumerateStep::Action,
            ) => Some(TracepointEnumerateCursor::Action { tp, addr, step: 0 }),
            (
                Some(&TracepointEnumerateCursor::Source { tp, addr, .. }),
                TracepointEnumerateStep::Action,
            ) => Some(TracepointEnumerateCursor::Action { tp, addr, step: 0 }),
            (
                Some(&TracepointEnumerateCursor::Action { tp, addr, step }),
                TracepointEnumerateStep::Action,
            ) => Some(TracepointEnumerateCursor::Action {
                tp,
                addr,
                step: step + 1,
            }),

            // Transition to enumerating sources
            (
                Some(
                    &TracepointEnumerateCursor::New { tp, addr }
                    | &TracepointEnumerateCursor::Action { tp, addr, .. },
                ),
                TracepointEnumerateStep::Source,
            ) => Some(TracepointEnumerateCursor::Source { tp, addr, step: 0 }),
            (
                Some(&TracepointEnumerateCursor::Source { tp, addr, step }),
                TracepointEnumerateStep::Source,
            ) => Some(TracepointEnumerateCursor::Source {
                tp,
                addr,
                step: step + 1,
            }),

            // Transition to the next tracepoint
            (Some(_), TracepointEnumerateStep::Next { tp, addr }) => {
                Some(TracepointEnumerateCursor::New { tp, addr })
            }
        };
        state.cursor = next;

        Ok(())
    }
}
