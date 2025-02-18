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

/// Describes a new tracepoint. GDB may ask for the state of current
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

/// Source string fragment for a tracepoint. A tracepoint may have more than one
/// source string, such as being describes by one source string for the location
/// and another for the actions, or by GDB splitting a larger source string
/// into multiple fragments. GDB may ask for the source of current tracepoints,
/// which are described by this same structure.
#[derive(Debug)]
pub struct SourceTracepoint<'a, U> {
    /// The tracepoint that the source string is specifying.
    pub number: Tracepoint,
    /// The PC address of the tracepoint that the source string is specifying.
    pub addr: U,
    /// What type of information for this tracepoint the string fragment is
    /// about.
    pub kind: TracepointSourceType,
    /// The offset in bytes within the overall source string this fragment is
    /// within.
    pub start: u32,
    /// The total length of the overall source string this fragment is within.
    pub slen: u32,
    /// The data for this source string fragment.
    pub bytes: ManagedSlice<'a, u8>,
}

#[cfg(feature = "alloc")]
impl<'a, U: Copy> SourceTracepoint<'a, U> {
    /// Allocate an owned copy of this structure.
    pub fn get_owned<'b>(&self) -> SourceTracepoint<'b, U> {
        SourceTracepoint {
            number: self.number,
            addr: self.addr,
            kind: self.kind,
            start: self.start,
            slen: self.slen,
            bytes: ManagedSlice::Owned(self.bytes.to_owned()),
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, U: Copy> TracepointAction<'a, U> {
    /// Allocate an owned copy of this structure.
    pub fn get_owned<'b>(&self) -> TracepointAction<'b, U> {
        use core::ops::Deref;
        match self {
            TracepointAction::Registers { mask } => TracepointAction::Registers {
                mask: ManagedSlice::Owned(mask.deref().into()),
            },
            TracepointAction::Memory {
                basereg,
                offset,
                length,
            } => TracepointAction::Memory {
                basereg: *basereg,
                offset: *offset,
                length: *length,
            },
            TracepointAction::Expression { expr } => TracepointAction::Expression {
                expr: ManagedSlice::Owned(expr.deref().into()),
            },
        }
    }
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
    DisconnectedTracing(bool),

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
    FrameNumber(u64),
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
pub(crate) enum TracepointEnumerateCursor<U> {
    New { tp: Tracepoint, addr: U },
    Action { tp: Tracepoint, addr: U, step: u64 },
    Source { tp: Tracepoint, addr: U, step: u64 },
}

/// The current state of enumerating tracepoints. gdbstub uses it as an opaque
/// bookkeeping record for what information has already been reported when GDB
/// downloads tracepoints on attachment.
#[derive(Debug, Default)]
pub struct TracepointEnumerateState<U> {
    pub(crate) cursor: Option<TracepointEnumerateCursor<U>>,
}

/// How to transition the [`TracepointEnumerateState`] state machine after
/// reporting an item for tracepoint enumeration.
#[derive(Debug)]
pub enum TracepointEnumerateStep<U> {
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
    ///
    /// Targets should only return this transition if they implement
    /// [`Tracepoints::support_tracepoint_source`], or else it indicates an
    /// error and the state machine iteration will stop.
    Source,
    /// The current tracepoint is done being enumerated, and GDB should next
    /// enumerate a different one.
    Next {
        /// The next tracepoint to move to.
        tp: Tracepoint,
        /// The PC of the next tracepoint.
        addr: U,
    },
    /// All tracepoints have been enumerated, and the state machine is done.
    Done,
}

/// Target Extension - Provide tracepoints.
pub trait Tracepoints: Target {
    /// Clear any saved tracepoints and empty the trace frame buffer.
    fn tracepoints_init(&mut self) -> TargetResult<(), Self>;

    /// Begin creating a new tracepoint according to the description `tdp`.
    fn tracepoint_create_begin(
        &mut self,
        tdp: NewTracepoint<<Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;
    /// Configure an existing tracepoint, appending an additional action to its
    /// definition.
    ///
    /// This method will only ever be called in-between
    /// [`Tracepoints::tracepoint_create_begin`] and
    /// [`Tracepoints::tracepoint_create_complete`] invocations for a new
    /// tracepoint.
    fn tracepoint_create_continue(
        &mut self,
        tp: Tracepoint,
        action: &TracepointAction<'_, <Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;
    /// Complete the creation of a tracepoint. All of its actions are expected
    /// to have been received.
    ///
    /// This method will only ever be called after all of the
    /// [`Tracepoints::tracepoint_create_begin`] and
    /// [`Tracepoints::tracepoint_create_continue`] invocations for a new
    /// tracepoint.
    fn tracepoint_create_complete(&mut self, tp: Tracepoint) -> TargetResult<(), Self>;
    /// Request the status of tracepoint `tp` at address `addr`.
    ///
    /// Returns a [`TracepointStatus`] with the requested information.
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
    ///
    /// For the average trait implementations, this will look like:
    ///
    /// ```
    /// struct MyTarget {
    ///    tracepoint_enumerate_state: TracepointEnumerateState,
    ///    ...
    /// }
    ///
    /// impl MyTarget {
    ///    fn new() -> Self {
    ///        MyTarget {
    ///            tracepoint_enumerate_state: TracepointEnumerateState::default(),
    ///            ...
    ///        }
    ///    }
    /// }
    ///
    /// impl Tracepoints for MyTarget {
    ///    fn tracepoint_enumerate_state(
    ///        &mut self,
    ///    ) -> &mut TracepointEnumerateState<<Self::Arch as Arch>::Usize> {
    ///        &mut self.tracepoint_enumerate_state
    ///    }
    /// }
    /// ```
    fn tracepoint_enumerate_state(
        &mut self,
    ) -> &mut TracepointEnumerateState<<Self::Arch as Arch>::Usize>;

    /// Begin enumerating a new tracepoint. If `tp` is None, then the first
    /// tracepoint recorded should be reported via `f`, otherwise the requested
    /// tracepoint should be.
    ///
    /// After reporting a tracepoint, [`TracepointEnumerateStep`] describes what
    /// information is still available. Unlike tracepoint *creation*, which is
    /// driven by GDB, for *enumeration* it's the responsibility of the trait
    /// implementation to correctly sequence the state transitions so that
    /// GDB is able to enumerate all of the information for the tracepoints the
    /// implementation has saved via the various `tracepoint_enumerate_*`
    /// methods.
    ///
    /// For the average implementation, it should report the requested
    /// tracepoint, and then return
    /// [`TracepointEnumerateStep::Action`] to transition to reporting
    /// actions for the tracepoint. If the trait implements
    /// [`TracepointSource`], it can instead return
    /// [`TracepointEnumerateStep::Source`] to begin reporting source items
    /// instead.
    fn tracepoint_enumerate_start(
        &mut self,
        tp: Option<Tracepoint>,
        f: &mut dyn FnMut(&NewTracepoint<<Self::Arch as Arch>::Usize>),
    ) -> TargetResult<TracepointEnumerateStep<<Self::Arch as Arch>::Usize>, Self>;
    /// Enumerate an action attached to a tracepoint. `step` is which action
    /// item is being asked for, so that the implementation can respond with
    /// multiple items across multiple function calls. Each action should be
    /// reported via `f`.
    ///
    /// After reporting a tracepoint action, [`TracepointEnumerateStep`]
    /// describes what information will next be enumerated: this may be
    /// [`TracepointEnumerateStep::Action`] if there are more actions that
    /// still need to be reported, for example.
    fn tracepoint_enumerate_action(
        &mut self,
        tp: Tracepoint,
        step: u64,
        f: &mut dyn FnMut(&TracepointAction<'_, <Self::Arch as Arch>::Usize>),
    ) -> TargetResult<TracepointEnumerateStep<<Self::Arch as Arch>::Usize>, Self>;

    /// Reconfigure the trace buffer to include or modify an attribute.
    fn trace_buffer_configure(&mut self, config: TraceBufferConfig) -> TargetResult<(), Self>;

    /// Read up to `len` bytes from the trace buffer, starting at `offset`.
    /// The trace buffer is treated as a contiguous collection of traceframes,
    /// as per [GDB's trace file format](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Trace-File-Format.html).
    /// The function `f` should be called to report as many bytes from
    /// the trace buffer that were requested as possible.
    fn trace_buffer_request(
        &mut self,
        offset: u64,
        len: usize,
        f: &mut dyn FnMut(&mut [u8]),
    ) -> TargetResult<(), Self>;

    /// Return the status of the current trace experiment.
    fn trace_experiment_status(
        &self,
        report: &mut dyn FnMut(ExperimentStatus<'_>),
    ) -> TargetResult<(), Self>;
    /// List any statistical information for the current trace experiment, by
    /// calling `report` with each [`ExperimentExplanation`] item.
    fn trace_experiment_info(
        &self,
        report: &mut dyn FnMut(ExperimentExplanation<'_>),
    ) -> TargetResult<(), Self>;
    /// Start a new trace experiment.
    fn trace_experiment_start(&mut self) -> TargetResult<(), Self>;
    /// Stop the currently running trace experiment.
    fn trace_experiment_stop(&mut self) -> TargetResult<(), Self>;

    /// Select a new frame in the trace buffer. The target should attempt to
    /// fulfill the request according to the [`FrameRequest`]. If it's
    /// successful it should call `report` with a series of calls describing
    /// the found frame, and then record it as the currently selected frame.
    /// Future register and memory requests should be fulfilled from the
    /// currently selected frame.
    fn select_frame(
        &mut self,
        frame: FrameRequest<<Self::Arch as Arch>::Usize>,
        report: &mut dyn FnMut(FrameDescription),
    ) -> TargetResult<(), Self>;

    /// Support for setting and enumerating the source strings for tracepoint
    /// actions.
    #[inline(always)]
    fn support_tracepoint_source(&mut self) -> Option<TracepointSourceOps<'_, Self>> {
        None
    }
}

/// Target Extension - Support setting and enumerating source strings for
/// tracepoint actions.
///
/// GDB requires source strings to be accurately reported back to it when it
/// attaches to a target in order to download tracepoints, or else it will
/// locally not be able to parse them and throw away the attached actions.
pub trait TracepointSource: Tracepoints {
    /// Configure an existing tracepoint, appending a new source string
    /// fragment.
    fn tracepoint_attach_source(
        &mut self,
        src: SourceTracepoint<'_, <Self::Arch as Arch>::Usize>,
    ) -> TargetResult<(), Self>;

    /// Enumerate the source strings that describe a tracepoint. `step` is which
    /// source string is being asked for, so that the implementation can
    /// respond with multiple items across multiple function calls. Each
    /// source string should be reported via `f`.
    ///
    /// After reporting a tracepoint source string, [`TracepointEnumerateStep`]
    /// describes what source string will next be enumerated.
    fn tracepoint_enumerate_source(
        &mut self,
        tp: Tracepoint,
        step: u64,
        f: &mut dyn FnMut(&SourceTracepoint<'_, <Self::Arch as Arch>::Usize>),
    ) -> TargetResult<TracepointEnumerateStep<<Self::Arch as Arch>::Usize>, Self>;
}

define_ext!(TracepointsOps, Tracepoints);
define_ext!(TracepointSourceOps, TracepointSource);
