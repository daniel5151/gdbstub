You may also find it useful to refer to the in-tree `armv4t` and `armv4t_multicore` examples when transitioning between versions.

# `0.4` -> `0.5`

While the overall structure of the API has remained the same, `0.5.0` does introduce a few breaking API changes that require some attention. That being said, it should not be a difficult migration, and updating to `0.5.0` from `0.4` shouldn't take more than 10 mins of refactoring.

Check out [`CHANGELOG.md`](../CHANGELOG.md) for a full list of changes.

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

impl SingleThreadOps for Emu {
    fn read_register(&mut self, reg_id: arch::arm::reg::id::ArmCoreRegId, dst: &mut [u8]) -> TargetResult<(), Self> { ... }
    fn write_register(&mut self, reg_id: arch::arm::reg::id::ArmCoreRegId, val: &[u8]) -> TargetResult<(), Self> { ... }
}

// ==== 0.5.0 ==== //

impl SingleThreadOps for Emu {
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
