//! Provide tracepoints for the target.
use crate::target::Arch;
use crate::target::Target;
use crate::target::TargetResult;
use managed::ManagedSlice;

/// A tracepoint, identified by a unique number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tracepoint(pub usize);

/// A state variable, identified by a unique number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateVariable(usize);

/// Describes a new tracepoint. It may be configured by later
/// [DefineTracepoint] structs. GDB may ask for the state of current
/// tracepoints, which are described with this same structure.
#[derive(Debug, Clone)]
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

/// Describes how to collect information for a trace frame when the tracepoint
/// it is attached to is hit. A tracepoint may have more than one action
/// attached.
#[derive(Debug)]
pub enum TracepointAction<'a, U> {
    /// Collect registers.
    Registers {
        /// A bitmask of which registers should be collected. The least
        /// significant bit is numberered zero. Note that the mask may
        /// be larger than the word length.
        mask: ManagedSlice<'a, u8>,
    },
    /// Collect memory.`len` bytes of memory starting at the address in register
    /// number `basereg`, plus `offset`. If `basereg` is None, then treat it
    /// as a fixed address.
    Memory {
        /// If `Some`, then calculate the address of memory to collect relative
        /// to the value of this register number. If `None` then memory
        /// should be collected from a fixed address.
        basereg: Option<u64>,
        /// The offset used to calculate the address to collect memory from.
        offset: U,
        /// How many bytes of memory to collect.
        length: u64,
    },
    /// Collect data according to an agent bytecode program.
    Expression {
        /// The GDB agent bytecode program to evaluate.
        expr: ManagedSlice<'a, u8>,
    },
}

/// A list of TracepointActions.
#[derive(Debug)]
pub(crate) enum TracepointActionList<'a, U> {
    /// Raw and unparsed actions, such as from GDB.
    Raw {
        /// The unparsed action data.
        data: ManagedSlice<'a, u8>,
    },
    /// A slice of parsed actions, such as what may be returned by a target when
    /// enumerating tracepoints.
    #[cfg(feature = "alloc")]
    Parsed {
        /// The parsed actions.
        actions: ManagedSlice<'a, TracepointAction<'a, U>>,
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
    pub(crate) actions: TracepointActionList<'a, U>,
}

/// What type of information a tracepoint source item is about.
#[derive(Debug, Clone, Copy)]
pub enum TracepointSourceType {
    /// Describes the location the tracepoint is at.
    At,
    /// Describes the conditional expression for a tracepoint.
    Cond,
    /// Describes the action command that should be executed when a tracepoint
    /// is hit.
    Cmd,
}

#[derive(Debug)]
/// Source string fragment for a tracepoint. A tracepoint may have more than one
/// source string, such as being describes by one source string for the location
/// and another for the actions, or by GDB splitting a larger source string
/// into multiple fragments. GDB may ask for the source of current tracepoints,
/// which are described by this same structure.
pub struct SourceTracepoint<'a, U> {
    /// The tracepoint that the source string is specifying.
    pub number: Tracepoint,
    /// The PC address of the tracepoint that the source string is specifying.
    pub addr: U,
    /// What type of information for this tracepoint the string fragment is
    /// about.
    pub r#type: TracepointSourceType,
    /// The offset in bytes within the overall source string this fragment is
    /// within.
    pub start: u32,
    /// The total length of the overall source string this fragment is within.
    pub slen: u32,
    /// The data for this source string fragment.
    pub bytes: ManagedSlice<'a, u8>,
}

/// An item from a stream of tracepoint descriptions. Enumerating tracepoints
/// should emit a sequence of Create and Define items for all the tracepoints
/// that are loaded.
#[derive(Debug)]
pub enum TracepointItem<'a, U> {
    /// Introduce a new tracepoint and describe its properties. This must be
    /// emitted before any [TracepointItem::Define] items that use the same
    /// tracepoint number, and must have the `more` flag set if it will be
    /// followed by [TracepointItem::Define] items for this tracepoint.
    New(NewTracepoint<U>),
    /// Define additional data for a tracepoint. This must be emitted after a
    /// [TracepointItem::New] item that introduces the tracepoint number, and
    /// must have the `more` flag set if it will be followed by more
    /// [TracepointItem::Define] items for this tracepoint.
    Define(DefineTracepoint<'a, U>),
}

/// The running state of a trace experiment.
#[derive(Debug)]
pub enum ExperimentStatus<'a> {
    /// The experiment is currently running
    Running,
    /// The experiment is not currently running, with no more information given
    /// as to why.
    NotRunning,
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
}

/// An explanation of some detail of the currently running trace experiment.
#[derive(Debug)]
pub enum ExperimentExplanation<'a> {
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
    Other(&'a str),
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
pub enum TraceBufferConfig {
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

/// Describes a detail of a frame from the trace buffer
#[derive(Debug)]
pub enum FrameDescription {
    /// The frame is at the specified index in the trace buffer
    FrameNumber(Option<u64>),
    /// The frame is a hit of the specified tracepoint
    Hit(Tracepoint),
}

/// The state of a tracepoint.
#[derive(Debug)]
pub struct TracepointStatus {
    /// The number of times a tracepoint has been hit in a trace run.
    pub hit_count: u64,
    /// The number of bytes the tracepoint accounts for in the trace buffer.
    pub bytes_used: u64,
}

#[derive(Debug)]
pub(crate) enum TracepointEnumerateCursor {
    New(Tracepoint),
    Action(Tracepoint, u64),
    Source(Tracepoint, u64),
}

/// The current state of enumerating tracepoints. gdbstub uses it as an opaque
/// bookkeeping record for what information has already been reported when GDB
/// downloads tracepoints on attachment.
#[derive(Debug, Default)]
pub struct TracepointEnumerateState {
    pub(crate) cursor: Option<TracepointEnumerateCursor>,
}

/// How to transition the [TracepointEnumerateState] state machine after
/// reporting an item for tracepoint enumeration.
#[derive(Debug)]
pub enum TracepointEnumerateStep {
    /// The current tracepoint that is being enumerated has more actions.
    ///
    /// Increments the step counter if the state machine was already
    /// enumerating actions, otherwise it is reset to 0 and GDB will start
    /// enumerating actions.
    Action,
    /// The current tracepoint that is being enumerated has more source strings.
    ///
    /// Increments the step counter if the state machine was already
    /// enumerating sources strings, otherwise it is reset to 0 and GDB will
    /// start enumerating source strings.
    Source,
    /// The current tracepoint has been enumerated, and GDB should next
    /// enumerate a different one.
    Next(Tracepoint),
    /// All tracepoints have been enumerated, and the state machine is done.
    Done,
}

/// Target Extension - Provide tracepoints.
pub trait Tracepoints: Target {
    /// Clear any saved tracepoints and empty the trace frame buffer.
    fn tracepoints_init(&mut self) -> TargetResult<(), Self>;

    /// Create a new tracepoint according to the description `tdp`.
    fn tracepoint_create(
        &mut self,
        tdp: NewTracepoint<<Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;
    /// Configure an existing tracepoint, appending new actions.
    fn tracepoint_define(
        &mut self,
        dtdp: DefineTracepoint<'_, <Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;
    /// Configure an existing tracepoint, appending a new source string
    /// fragment.
    fn tracepoint_attach_source(
        &mut self,
        src: SourceTracepoint<'_, <Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;
    /// Request the status of tracepoint `tp` at address `addr`.
    ///
    /// Returns a [TracepointStatus] with the requested information.
    fn tracepoint_status(
        &self,
        tp: Tracepoint,
        addr: <Self::Arch as Arch>::Usize,
    ) -> TargetResult<TracepointStatus, Self>;

    /// Return the stub's tracepoint enumeration state. gdbstub internally
    /// uses this state to support GDB downloading tracepoints on attachment,
    /// but requires the target implementation to provide storage for it.
    ///
    /// The state instance that this returns should be the same across multiple
    /// calls and unmodified, or else gdbstub will be unable to transition the
    /// state machine during enumeration correctly.
    fn tracepoint_enumerate_state(&mut self) -> &mut TracepointEnumerateState;

    /// Begin enumerating a new tracepoint. If `tp` is None, then the first
    /// tracepoint recorded should be reported via `f`, otherwise the requested
    /// tracepoint should be.
    ///
    /// After reporting a tracepoint, [TracepointEnumerateStep] describes what
    /// information is still available.
    fn tracepoint_enumerate_start(
        &mut self,
        tp: Option<Tracepoint>,
        f: &mut dyn FnMut(NewTracepoint<<Self::Arch as Arch>::Usize>),
    ) -> TargetResult<TracepointEnumerateStep, Self>;
    /// Enumerate the actions attached to a tracepoint. `step` is which action
    /// item is being asked for, so that the implementation can respond with
    /// multiple items across multiple function calls. Each action should be
    /// reported via `f`.
    ///
    /// After reporting a tracepoint action, [TracepointEnumerateStep] describes
    /// what information will next be enumerated.
    fn tracepoint_enumerate_action(
        &mut self,
        tp: Tracepoint,
        step: u64,
        f: &mut dyn FnMut(DefineTracepoint<'_, <Self::Arch as Arch>::Usize>),
    ) -> TargetResult<TracepointEnumerateStep, Self>;
    /// Enumerate the source strings that describe a tracepoint. `step` is which
    /// source string is being asked for, so that the implementation can
    /// respond with multiple items across multiple function calls. Each
    /// source string should be reported via `f`.
    ///
    /// After reporting a tracepoint source string, [TracepointEnumerateStep]
    /// describes what source string will next be enumerated.
    fn tracepoint_enumerate_source(
        &mut self,
        tp: Tracepoint,
        step: u64,
        f: &mut dyn FnMut(SourceTracepoint<'_, <Self::Arch as Arch>::Usize>),
    ) -> TargetResult<TracepointEnumerateStep, Self>;

    /// Reconfigure the trace buffer to include or modify an attribute.
    fn trace_buffer_configure(&mut self, config: TraceBufferConfig) -> TargetResult<(), Self>;

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
    /// List any statistical information for the current trace experiment, by
    /// calling `report` with each [ExperimentExplanation] item.
    fn trace_experiment_info(
        &self,
        report: &mut dyn FnMut(ExperimentExplanation<'_>),
    ) -> TargetResult<(), Self>;
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
