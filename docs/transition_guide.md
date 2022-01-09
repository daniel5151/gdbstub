# Transition Guide

This document provides a brief overview of breaking changes between major `gdbstub` releases, along with tips/tricks/suggestions on how to migrate between `gdbstub` releases.

This document does _not_ discuss any new features that might have been added between releases. For a comprehensive overview of what's been _added_ to `gdbstub` (as opposed to what's _changed_), check out the [`CHANGELOG.md`](../CHANGELOG.md).

> _Note:_ after reading through this doc, you may also find it helpful to refer to the in-tree `armv4t` and `armv4t_multicore` examples when transitioning between versions.

## `0.5` -> `0.6`

`0.6` introduces a large number of breaking changes to the public APIs, and will require quite a bit more more "hands on" porting than previous `gdbstub` upgrades.

The following guide is a best-effort attempt to document all the changes, but there are some parts that may be missing / incomplete.

##### General API change - _lots_ of renaming + exported type reorganization

Many types have been renamed, and many import paths have changed in `0.6`.

Exhaustively listing them would be nearly impossible, but suffice it to say, you will need to tweak your imports.

##### `Connection` API changes

> _Note:_ If you haven't implemented `Connection` yourself (i.e: you are using one of the built-in `Connection` impls on `TcpStream`/`UnixStream`), you can skip this section.

The blocking `read` method and non-blocking `peek` methods have been removed from the base `Connection` API, and have been moved to a new `ConnectionExt` type.

For more context around this change, please refer to [Moving from `GdbStub::run` to `GdbStub::run_blocking`](#moving-from-gdbstubrun-to-gdbstubrun_blocking).

Porting a `0.5` `Connection` to `0.6` is incredibly straightforward - you simply split your existing implementation in two:

```rust
// ==== 0.5.x ==== //

impl Connection for MyConnection {
    type Error = MyError;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> { .. }
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> { .. }
    fn read(&mut self) -> Result<u8, Self::Error> { .. }
    fn peek(&mut self) -> Result<Option<u8>, Self::Error> { .. }
    fn flush(&mut self) -> Result<(), Self::Error> { .. }
    fn on_session_start(&mut self) -> Result<(), Self::Error> { .. }
}

// ==== 0.6.0 ==== //

impl Connection for MyConnection {
    type Error = MyError;

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> { .. }
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> { .. }
    fn flush(&mut self) -> Result<(), Self::Error> { .. }
    fn on_session_start(&mut self) -> Result<(), Self::Error> { .. }
}

impl ConnectionExt for MyConnection {
    type Error = MyError;

    fn read(&mut self) -> Result<u8, Self::Error> { .. }
    fn peek(&mut self) -> Result<Option<u8>, Self::Error> { .. }
}

```

##### `Arch` API - `RegId::from_raw_id`

> _Note:_ If you haven't implemented `Arch` yourself (i.e: you are any of the `Arch` impls from `gdbstub_arch`), you can skip this section.

The `Arch` API has had one breaking changes: The `RegId::from_raw_id` method's "register size" return value has been changed from `usize` to `Option<NonZeroUsize>`.

If the register size is `Some`, `gdbstub` will include a runtime check to ensures that the target implementation does not send back more bytes than the register allows when responding to single-register read requests.

If the register size is `None`, `gdbstub` will _omit_ this runtime check, and trust that the target's implementation of `read_register` is correct.

_Porting advice:_ If your `Arch` implementation targets a specific architecture, it is _highly recommended_ that you simply wrap your existing size value with `Some`. This API change was made to support dynamic `Arch` implementations, whereby the behavior of the `Arch` varies on the runtime state of the program (e.g: in multi-system emulators), and there is not "fixed" register size per id.

##### `Target` API - IDET methods are now prefixed with `supports_`

All IDET methods have been prefixed with `supports_`, to make it easier to tell at-a-glance which methods are actual handler methods, and which are simply IDET plumbing.

As such, when porting target code from `0.5` to `0.6`, before you dive into any functional changes, you should take a moment to find and rename any methods that have had their name changed.

##### `Target` API - Introducing `enum Signal`

In prior versions of `gdbstub`, signals were encoded as raw `u8` values. This wasn't very user-friendly, as it meant users had to manually locate the signal-to-integer mapping table themselves when working with signals in code.

`0.6` introduces a new `enum Signal` which encodes this information within `gdbstub` itself.

This new `Signal` type has replaced `u8` in any places that a `u8` was used to represent a signal, such as in `StopReason::Signal`, or as part of the various `resume` APIs.

_Porting advice:_ The Rust compiler should catch any type errors due to this change, making it easy to swap out any instances of `u8` with the new `Signal` type.

##### `HwWatchpoint` API - Plumb watchpoint `length` parameter to public API

The watchpoint API has been updated to include a new `length` parameter, specifying what range of memory addresses the watchpoint should encompass.

##### `TargetXmlOverride` API - Return data via `&mut [u8]` buffer

In an effort to unify the implementations of various new `qXfer`-backed protocol extensions, the existing `TargetXmlOverride` has been changed from returning a `&str` value to using a `std::io::Read`-style "write the data into a `&mut [u8]` buffer" API.

Porting a `0.5` `TargetDescriptionXmlOverride` to `0.6` is straightforward, though a bit boilerplate-y.

```rust
// ==== 0.5.x ==== //

impl target::ext::target_description_xml_override::TargetDescriptionXmlOverride for Emu {
    fn target_description_xml(&self) -> &str {
        r#"<target version="1.0"><!-- custom override string --><architecture>armv4t</architecture></target>"#
    }
}

// ==== 0.6.0 ==== //

pub fn copy_to_buf(data: &[u8], buf: &mut [u8]) -> usize {
    let len = data.len();
    let buf = &mut buf[..len];
    buf.copy_from_slice(data);
    len
}

pub fn copy_range_to_buf(data: &[u8], offset: u64, length: usize, buf: &mut [u8]) -> usize {
    let offset = match usize::try_from(offset) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let len = data.len();
    let data = &data[len.min(offset)..len.min(offset + length)];
    copy_to_buf(data, buf)
}

impl target::ext::target_description_xml_override::TargetDescriptionXmlOverride for Emu {
    fn target_description_xml(
        &self,
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let xml = r#"<target version="1.0"><!-- custom override string --><architecture>armv4t</architecture></target>"#
            .trim()
            .as_bytes();
        Ok(copy_range_to_buf(xml, offset, length, buf))
    }
}
```

##### Updates to `{Single,Multi}ThreadOps::resume` API

`0.6` includes three fairly major behavioral changes to the `resume` method:

###### Support for `resume` is now entirely optional

There are quite a few use cases where it might make sense to debug a target that does _not_ support resumption, e.g: a post-mortem debugging session, or when debugging crash dumps. In these cases, past version of `gdbstub` would force the user to nonetheless implement "stub" methods for resuming these targets, along with forcing users to pay the "cost" of including all the handler code related to resumption (of which there is quite a bit.)

In `0.6`, all resume-related functionality has been extracted out of `{Single,Multi}ThreadBase`, and split into new `{Singe,Multi}ThreadResume` IDETs.

###### Removing `ResumeAction`, and making single-step support optional

The GDB protocol only requires that targets implement support for _continuing_ execution - support for instruction-level single-step execution is totally optional.

> Note: this isn't actually true in practice, thanks to a bug in the mainline GDB client... See the docs for `Target::use_optional_single_step` for details...

To model this behavior, `0.6` has split single-step support into its own IDET, in a manner similar to how optimized range step support was handled in `0.5`.

In doing so, the `enum ResumeAction` type could be removed entirely, as single-step resume was to be handled in its own method.

###### Removing `gdb_interrupt: GdbInterrupt`, and making `resume` non-blocking

In past versions of `gdbstub`, the `resume` API would _block_ the thread waiting for the target to hit some kind of stop condition. In this model, checking for pending GDB interrupts was quite unergonomic, requiring that the thread periodically wake up and check whether an interrupt has arrived via the `GdbInterrupt` type.

`gdbstub` `0.6` introduces a new paradigm of driving target execution, predicated on the idea that the target's `resume` method _does not block_, instead yielding execution immediately, and deferring the responsibility of "selecting" between incoming stop events and GDB interrupts to higher levels of the `gdbstub` "stack".

In practice, this means that much of the logic that used to live in the `resume` implementation will now move into upper-levels of the `gdbstub` API, with the `resume` API serving more of a "bookkeeping" purpose, recording what kind of resumption mode the GDB client has requested from the target, while not actually resuming the target itself.

For more context around this change, please refer to [Moving from `GdbStub::run` to `GdbStub::run_blocking`](#moving-from-gdbstubrun-to-gdbstubrun_blocking).

###### Example: migrating `resume` from `0.5` to `0.6`

Much of the code contained within methods such as `block_until_stop_reason_or_interrupt` will be lifted into upper layers of the `gdbstub` API, leaving behind just a small bit of code in the target's `resume` method to perform "bookkeeping" regarding how the GDB client requested the target to be resumed.

```rust
// ==== 0.5.x ==== //

impl SingleThreadBase for Emu {
    fn resume(
        &mut self,
        action: ResumeAction,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<StopReason<u32>, Self::Error> {
        match action {
            ResumeAction::Step => self.do_single_step(),
            ResumeAction::Continue => self.block_until_stop_reason_or_interrupt(action, || gdb_interrupt.pending()),
            _ => self.handle_resume_with_signal(action),
        }
    }
}

// ==== 0.6.0 ==== //

impl SingleThreadBase for Emu {
    // resume has been split into a separate IDET
    #[inline(always)]
    fn support_resume(
        &mut self
    ) -> Option<SingleThreadResumeOps<Self>> {
        Some(self)
    }
}


impl SingleThreadResume for Emu {
    fn resume(
        &mut self,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> { // <-- no longer returns a stop reason!
        if let Some(signal) = signal {
            self.handle_signal(signal)?;
        }

        // upper layers of the `gdbstub` API will be responsible for "driving"
        // target execution - `resume` simply performs book keeping on _how_ the
        // target should be resumed.
        self.set_execution_mode(ExecMode::Continue)?;

        Ok(())
    }

    // single-step support has been split into a separate IDET
    #[inline(always)]
    fn support_single_step(
        &mut self
    ) -> Option<SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadSingleStep for Emu {
    fn step(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        if let Some(signal) = signal {
            self.handle_signal(signal)?;
        }

        self.set_execution_mode(ExecMode::Step)?;
        Ok(())
    }
}
```

##### Moving from `GdbStub::run` to `GdbStub::run_blocking`

With the introduction of the new state-machine API, the responsibility of reading incoming has been lifted out of `gdbstub` itself, and is now something implementations are responsible for . The alternative approach would've been to have `Connection` include multiple different `read`-like methods for various kinds of paradigms - such as `async`/`await`, `epoll`, etc...

> TODO. In the meantime, I would suggest looking at rustdoc for details on how to use `GdbStub::run_blocking`...

## `0.4` -> `0.5`

While the overall structure of the API has remained the same, `0.5.0` does introduce a few breaking API changes that require some attention. That being said, it should not be a difficult migration, and updating to `0.5.0` from `0.4` shouldn't take more than 10 mins of refactoring.

##### Consolidating the `{Hw,Sw}Breakpoint/Watchpoint` IDETs under the newly added `Breakpoints` IDETs.

The various breakpoint IDETs that were previously directly implemented on the top-level `Target` trait have now been consolidated under a single `Breakpoints` IDET. This is purely an organizational change, and will not require rewriting any existing `{add, remove}_{sw_break,hw_break,watch}point` implementations.

Porting from `0.4` to `0.5` should be as simple as:

```rust
// ==== 0.4.x ==== //

impl Target for Emu {
    fn sw_breakpoint(&mut self) -> Option<target::ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }

    fn hw_watchpoint(&mut self) -> Option<target::ext::breakpoints::HwWatchpointOps<Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::SwBreakpoint for Emu {
    fn add_sw_breakpoint(&mut self, addr: u32) -> TargetResult<bool, Self> { ... }
    fn remove_sw_breakpoint(&mut self, addr: u32) -> TargetResult<bool, Self> { ... }
}

impl target::ext::breakpoints::HwWatchpoint for Emu {
    fn add_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> { ... }
    fn remove_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> { ... }
}

// ==== 0.5.0 ==== //

impl Target for Emu {
    // (New Method) //
    fn breakpoints(&mut self) -> Option<target::ext::breakpoints::BreakpointsOps<Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::Breakpoints for Emu {
    fn sw_breakpoint(&mut self) -> Option<target::ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }

    fn hw_watchpoint(&mut self) -> Option<target::ext::breakpoints::HwWatchpointOps<Self>> {
        Some(self)
    }
}

// (Almost Unchanged) //
impl target::ext::breakpoints::SwBreakpoint for Emu {
    //                                            /-- New `kind` parameter
    //                                           \/
    fn add_sw_breakpoint(&mut self, addr: u32, _kind: arch::arm::ArmBreakpointKind) -> TargetResult<bool, Self> { ... }
    fn remove_sw_breakpoint(&mut self, addr: u32, _kind: arch::arm::ArmBreakpointKind) -> TargetResult<bool, Self> { ... }
}

// (Unchanged) //
impl target::ext::breakpoints::HwWatchpoint for Emu {
    fn add_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> { ... }
    fn remove_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> { ... }
}

```

##### Single-register access methods (`{read,write}_register`) are now a separate `SingleRegisterAccess` trait

Single register access is not a required part of the GDB protocol, and as such, has been moved out into its own IDET. This is a purely organizational change, and will not require rewriting any existing `{read,write}_register` implementations.

Porting from `0.4` to `0.5` should be as simple as:

```rust
// ==== 0.4.x ==== //

impl SingleThreadResume for Emu {
    fn read_register(&mut self, reg_id: arch::arm::reg::id::ArmCoreRegId, dst: &mut [u8]) -> TargetResult<(), Self> { ... }
    fn write_register(&mut self, reg_id: arch::arm::reg::id::ArmCoreRegId, val: &[u8]) -> TargetResult<(), Self> { ... }
}

// ==== 0.5.0 ==== //

impl SingleThreadResume for Emu {
    // (New Method) //
    fn single_register_access(&mut self) -> Option<target::ext::base::SingleRegisterAccessOps<(), Self>> {
        Some(self)
    }
}

impl target::ext::base::SingleRegisterAccess<()> for Emu {
    //                           /-- New `tid` parameter (ignored on single-threaded systems)
    //                          \/
    fn read_register(&mut self, _tid: (), reg_id: arch::arm::reg::id::ArmCoreRegId, dst: &mut [u8]) -> TargetResult<(), Self> { ... }
    fn write_register(&mut self, _tid: (), reg_id: arch::arm::reg::id::ArmCoreRegId, val: &[u8]) -> TargetResult<(), Self> { ... }
}
```

##### New `MultiThreadOps::resume` API

In `0.4`, resuming a multithreaded target was done using an `Actions` iterator passed to a single `resume` method. In hindsight, this approach had a couple issues:

- It was impossible to statically enforce the property that the `Actions` iterator was guaranteed to return at least one element, often forcing users to manually `unwrap`
- The iterator machinery was quite heavy, and did not optimize very effectively
- Handling malformed packets encountered during iteration was tricky, as the user-facing API exposed an infallible iterator, thereby complicating the internal error handling
- Adding new kinds of `ResumeAction` (e.g: range stepping) required a breaking change, and forced users to change their `resume` method implementation regardless whether or not their target ended up using said action.

In `0.5`, the API has been refactored to address some of these issues, and the single `resume` method has now been split into multiple "lifecycle" methods:

1. `resume`
    - As before, when `resume` is called the target should resume execution.
    - But how does the target know how each thread should be resumed? That's where the next method comes in...
1. `set_resume_action`
    - This method is called prior to `resume`, and notifies the target how a particular `Tid` should be resumed.
1. (optionally) `set_resume_action_range_step`
    - If the target supports optimized range-stepping, it can opt to implement the newly added `MultiThreadRangeStepping` IDET which includes this method.
    - Targets that aren't interested in optimized range-stepping can skip this method!
1. `clear_resume_actions`
    - After the target returns a `ThreadStopReason` from `resume`, this method will be called to reset the previously set per-`tid` resume actions.

NOTE: This change does mean that targets are now responsible for maintaining some internal state that maps `Tid`s to `ResumeAction`s. Thankfully, this isn't difficult at all, and can as simple as maintaining a `HashMap<Tid, ResumeAction>`.

Please refer to the in-tree `armv4t_multicore` example for an example of how this new `resume` flow works.
