//! Provide tracepoints for the target.
use crate::conn::Connection;
use crate::protocol::ResponseWriter;
use crate::protocol::ResponseWriterError;
use crate::target::Arch;
use crate::target::Target;
use crate::target::TargetResult;
use managed::ManagedSlice;
use num_traits::PrimInt;

/// A tracepoint, identified by a unique number.
#[derive(Debug, Clone, Copy)]
pub struct Tracepoint(pub usize);

/// A state variable, identified by a unique number.
#[derive(Debug, Clone, Copy)]
pub struct StateVariable(usize);

/// Describes a new tracepoint. It may be configured by later
/// [DefineTracepoint] structs. GDB may ask for the state of current
/// tracepoints, which are described with this same structure.
#[derive(Debug)]
pub struct NewTracepoint<U> {
    /// The tracepoint number
    pub number: Tracepoint,
    /// If the tracepoint is enabled or not
    pub enabled: bool,
    /// The address the tracepoint is set at.
    pub addr: U,
    /// The tracepoint's step count
    pub step_count: u64,
    /// The tracepoint's pass count.
    pub pass_count: u64,
    /// If there will be tracepoint "define" packets that follow this.
    pub more: bool,
}

impl<U: crate::internal::BeBytes + num_traits::Zero + PrimInt> NewTracepoint<U> {
    pub(crate) fn write<C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), ResponseWriterError<C::Error>> {
        res.write_str("QTDP:")?;
        res.write_num(self.number.0)?;
        res.write_str(":")?;
        let mut buf = [0; 8];
        self.addr
            .to_be_bytes(&mut buf)
            .ok_or_else(|| unreachable!())?;
        res.write_hex_buf(&buf)?;
        res.write_str(":")?;
        res.write_str(if self.enabled { "E" } else { "D" })?;
        res.write_str(":")?;
        res.write_num(self.step_count)?;
        res.write_str(":")?;
        res.write_num(self.pass_count)?;

        Ok(())
    }
}

/// Describes how to collect information for a trace frame when the tracepoint
/// it is attached to is hit. A tracepoint may have more than one action
/// attached.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum TracepointAction<'a, U> {
    /// Collect the registers whose bits are set in `mask` (big endian).
    /// Note that `mask` may be larger than the word length.
    Registers { mask: &'a [u8] },
    /// Collect `len` bytes of memory starting at the address in register number
    /// `basereg`, plus `offset`. If `basereg` is None, then treat it as a fixed
    /// address.
    Memory {
        basereg: Option<u64>,
        offset: U,
        length: u64,
    },
    /// Evaluate `expr`, which is a GDB agent bytecode expression, and collect
    /// memory as it directs.
    Expression { expr: &'a [u8] },
}

impl<'a, U: crate::internal::BeBytes + num_traits::Zero + PrimInt> TracepointAction<'a, U> {
    pub(crate) fn write<C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), ResponseWriterError<C::Error>> {
        match self {
            TracepointAction::Registers { mask } => {
                res.write_str("R ")?;
                res.write_hex_buf(mask)?;
            }
            TracepointAction::Memory {
                basereg,
                offset,
                length,
            } => {
                res.write_str("M ")?;
                match basereg {
                    Some(r) => res.write_num(*r),
                    None => res.write_str("-1"),
                }?;
                res.write_str(",")?;
                let mut buf = [0; 8];
                offset.to_be_bytes(&mut buf).ok_or_else(|| unreachable!())?;
                res.write_hex_buf(&buf)?;
                res.write_str(",")?;
                res.write_num(*length)?;
            }
            TracepointAction::Expression { expr } => {
                res.write_str("X ")?;
                res.write_num(expr.len())?;
                res.write_str(",")?;
                res.write_hex_buf(expr)?;
            }
        }
        Ok(())
    }
}

/// A list of TracepointActions, either raw and unparsed from a GDB packet, or
/// a slice of parsed structures like which may be returned from enumerating
/// tracepoints.
#[derive(Debug)]
#[allow(missing_docs)]
pub enum TracepointActionList<'a, U> {
    /// Raw and unparsed actions, such as from GDB.
    Raw { data: &'a mut [u8] },
    /// A slice of parsed actions, such as what may be returned by a target when
    /// enumerating tracepoints. `more` must be set if there will be another
    /// "tracepoint definition" with more actions for this tracepoint.
    Parsed {
        actions: ManagedSlice<'a, TracepointAction<'a, U>>,
        more: bool,
    },
}

/// Definition data for a tracepoint. A tracepoint may have more than one define
/// structure for all of its data. GDB may ask for the state of current
/// tracepoints, which are described with this same structure.
#[derive(Debug)]
pub struct DefineTracepoint<'a, U> {
    /// The tracepoint that is having actions appended to its definition.
    pub number: Tracepoint,
    /// The PC address of the tracepoint that is being defined.
    pub addr: U,
    /// A list of actions that should be appended to the tracepoint.
    pub actions: TracepointActionList<'a, U>,
}

impl<'a, U: crate::internal::BeBytes + num_traits::Zero + PrimInt> DefineTracepoint<'a, U> {
    pub(crate) fn write<C: Connection>(
        self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), ResponseWriterError<C::Error>> {
        res.write_str("QTDP:-")?;
        res.write_num(self.number.0)?;
        res.write_str(":")?;
        let mut buf = [0; 8];
        self.addr
            .to_be_bytes(&mut buf)
            .ok_or_else(|| unreachable!())?;
        res.write_hex_buf(&buf)?;
        res.write_str(":")?;
        let mut err = None;
        let more = self.actions(|action| {
            if let Err(e) = action.write(res) {
                err = Some(e)
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        if let Some(true) = more {
            res.write_str("-")?;
        }

        Ok(())
    }
}

/// An item from a stream of tracepoint descriptions. Enumerating tracepoints
/// should emit a sequence of Create and Define items for all the tracepoints
/// that are loaded.
#[derive(Debug)]
pub enum TracepointItem<'a, U> {
    /// Introduce a new tracepoint and describe its properties. This must be
    /// emitted before any [TracepointItem::Define] items that use the same tracepoint
    /// number, and must have the `more` flag set if it will be followed by
    /// [TracepointItem::Define] items for this tracepoint.
    New(NewTracepoint<U>),
    /// Define additional data for a tracepoint. This must be emitted after a
    /// [TracepointItem::New] item that introduces the tracepoint number, and must have
    /// the `more` flag set if it will be followed by more [TracepointItem::Define] items
    /// for this tracepoint.
    Define(DefineTracepoint<'a, U>),
}

/// Description of the currently running trace experiment.
pub struct ExperimentStatus<'a> {
    /// If a trace is presently running
    pub running: bool,
    /// A list of optional explanations for the trace status.
    pub explanations: ManagedSlice<'a, ExperimentExplanation<'a>>,
}

/// An explanation of some detail of the currently running trace experiment.
#[derive(Debug)]
pub enum ExperimentExplanation<'a> {
    /// No trace has been ran yet.
    NotRun,
    /// The trace was stopped by the user. May contain an optional user-supplied
    /// stop reason.
    Stop(Option<&'a [u8]>),
    /// The trace stopped because the buffer is full.
    Full,
    /// The trace stopped because GDB disconnect from the target.
    Disconnected,
    /// The trace stopped because the specified tracepoint exceeded its pass
    /// count.
    PassCount(Tracepoint),
    /// The trace stopped because the specified tracepoint had an error.
    Error(&'a [u8], Tracepoint),
    /// The trace stopped for some other reason.
    Unknown,

    // Statistical information
    /// The number of trace frames in the buffer.
    Frames(usize),
    /// The total number of trace frames created during the run. This may be
    /// larger than the trace frame count, if the buffer is circular.
    Created(usize),
    /// The total size of the trace buffer, in bytes.
    Size(usize),
    /// The number of bytes still unused in the buffer.
    Free(usize),
    /// The value of the circular trace buffer flag. True means the trace buffer
    /// is circular and old trace frames will be discarded if necessary to
    /// make room, false means that the trace buffer is linear and may fill
    /// up.
    Circular(bool),
    /// The value of the disconnected tracing flag. True means that tracing will
    /// continue after GDB disconnects, false means that the trace run will
    /// stop.
    Disconn(bool),

    /// Report a raw string as a trace status explanation.
    Other(managed::Managed<'a, str>),
}

impl<'a> ExperimentExplanation<'a> {
    pub(crate) fn write<C: Connection>(
        &self,
        res: &mut ResponseWriter<'_, C>,
    ) -> Result<(), ResponseWriterError<C::Error>> {
        use ExperimentExplanation::*;
        match self {
            NotRun => res.write_str("tnotrun:0")?,
            Stop(ref t) => match t {
                Some(text) => {
                    res.write_str("tstop:")?;
                    res.write_hex_buf(text)?;
                    res.write_str(":0")?;
                }
                None => res.write_str("tstop:0")?,
            },
            Full => res.write_str("tfull:0")?,
            Disconnected => res.write_str("tdisconnected:0")?,
            PassCount(tpnum) => {
                res.write_str("tpasscount:")?;
                res.write_num(tpnum.0)?;
            }
            Error(text, tpnum) => {
                res.write_str("terror:")?;
                res.write_hex_buf(text)?;
                res.write_str(":")?;
                res.write_num(tpnum.0)?;
            }
            Unknown => res.write_str("tunknown:0")?,

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

/// Shape of the trace buffer
#[derive(Debug)]
pub enum BufferShape {
    /// A circular trace buffer
    Circular,
    /// A linear trace buffer
    Linear,
}

/// Configuration for the trace buffer.
#[derive(Debug)]
pub enum TraceBuffer {
    /// Set the buffer's shape.
    Shape(BufferShape),
    /// Set the buffer's size in bytes. If None, the target should use whatever
    /// size it prefers.
    Size(Option<u64>),
}

/// Request to select a new frame from the trace buffer.
#[derive(Debug)]
pub enum FrameRequest<U> {
    /// Select the specified tracepoint frame in the buffer.
    Select(u64),
    /// Select a tracepoint frame that has a specified PC after the currently
    /// selected frame.
    AtPC(U),
    /// Select a tracepoint frame that hit a specified tracepoint after the
    /// currently selected frame.
    Hit(Tracepoint),
    /// Select a tramepoint frame that has a PC between a start (inclusive) and
    /// end (inclusive).
    Between(U, U),
    /// Select a tracepoint frame that has a PC outside the range of addresses
    /// (exclusive).
    Outside(U, U),
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

/// Describes a detail of a frame from the trace buffer
#[derive(Debug)]
pub enum FrameDescription {
    /// The frame is at the specified index in the trace buffer
    FrameNumber(Option<u64>),
    /// The frame is a hit of the specified tracepoint
    Hit(Tracepoint),
}

/// Target Extension - Provide tracepoints.
pub trait Tracepoints: Target {
    /// Clear any saved tracepoints and empty the trace frame buffer
    fn tracepoints_init(&mut self) -> TargetResult<(), Self>;

    /// Create a new tracepoint according to the description `tdp`
    fn tracepoint_create(
        &mut self,
        tdp: NewTracepoint<<Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;
    /// Configure an existing tracepoint, appending new actions
    fn tracepoint_define(
        &mut self,
        dtdp: DefineTracepoint<'_, <Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;
    /// Request the status of tracepoint `tp` at address `addr`.
    ///
    /// Returns `(number of tracepoint hits, number of bytes used for frames)`.
    fn tracepoint_status(
        &self,
        tp: Tracepoint,
        addr: <Self::Arch as Arch>::Usize,
    ) -> TargetResult<(u64, u64), Self>;

    /// Begin enumerating tracepoints. The target implementation should
    /// initialize a state machine that is stepped by
    /// [Tracepoints::tracepoint_enumerate_step], and returns TracepointItems that
    /// correspond with the currently configured tracepoints.
    fn tracepoint_enumerate_start(
        &mut self,
    ) -> TargetResult<Option<TracepointItem<'_, <Self::Arch as Arch>::Usize>>, Self>;
    /// Step the tracepoint enumeration state machine. The target implementation
    /// should return TracepointItems that correspond with the currently
    /// configured tracepoints.
    fn tracepoint_enumerate_step(
        &mut self,
    ) -> TargetResult<Option<TracepointItem<'_, <Self::Arch as Arch>::Usize>>, Self>;

    /// Reconfigure the trace buffer to include or modify an attribute.
    fn trace_buffer_configure(&mut self, tb: TraceBuffer) -> TargetResult<(), Self>;

    /// Return up to `len` bytes from the trace buffer, starting at `offset`.
    /// The trace buffer is treated as a contiguous collection of traceframes,
    /// as per [GDB's trace file format](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Trace-File-Format.html).
    /// The return value should be the number of bytes written.
    fn trace_buffer_request(
        &mut self,
        offset: u64,
        len: usize,
        buf: &mut [u8],
    ) -> TargetResult<Option<usize>, Self>;

    /// Return the status of the current trace experiment.
    fn trace_experiment_status(&self) -> TargetResult<ExperimentStatus<'_>, Self>;
    /// Start a new trace experiment.
    fn trace_experiment_start(&mut self) -> TargetResult<(), Self>;
    /// Stop the currently running trace experiment.
    fn trace_experiment_stop(&mut self) -> TargetResult<(), Self>;

    /// Select a new frame in the trace buffer. The target should attempt to
    /// fulfill the request according to the [FrameRequest]. If it's
    /// successful it should call `report` with a series of calls describing
    /// the found frame, and then record it as the currently selected frame.
    /// Future register and memory requests should be fulfilled from the
    /// currently selected frame.
    fn select_frame(
        &mut self,
        frame: FrameRequest<<Self::Arch as Arch>::Usize>,
        report: &mut dyn FnMut(FrameDescription),
    ) -> TargetResult<(), Self>;
}

define_ext!(TracepointsOps, Tracepoints);
