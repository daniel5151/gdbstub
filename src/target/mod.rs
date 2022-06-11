//! The core [`Target`] trait, and all its various protocol extension traits.
//!
//! The [`Target`] trait describes how to control and modify a system's
//! execution state during a GDB debugging session, and serves as the
//! primary bridge between `gdbstub`'s generic protocol implementation and a
//! target's project/platform-specific code.
//!
//! **`Target` is the most important trait in `gdbstub`, and must be implemented
//! by all consumers of the library!**
//!
//! # Implementing `Target`
//!
//! `gdbstub` uses a technique called ["Inlineable Dyn Extension Traits"](ext)
//! (IDETs) to expose an ergonomic and extensible interface to the GDB protocol.
//! It's not a very common pattern, and can seem a little "weird" at first
//! glance, but IDETs are actually very straightforward to use!
//!
//! **TL;DR:** Whenever you see a method that returns something that looks like
//! `Option<ProtocolExtOps>`, you can enable that protocol extension by
//! implementing the `ProtocolExt` type on your target, and overriding the
//! `Option<ProtocolExtOps>` method to return `Some(self)`.
//!
//! Please refer to the [documentation in the `ext` module](ext) for more
//! information on IDETs, including a more in-depth explanation of how they
//! work, and how `Target` leverages them to provide fine grained control over
//! enabled protocol features.
//!
//! ## Associated Types
//!
//! - The [`Target::Arch`](trait.Target.html#associatedtype.Arch) associated
//!   type encodes information about the target's architecture, such as its
//!   pointer size, register layout, etc... `gdbstub` comes with several
//!   built-in architecture definitions, which can be found under the
//!   [`arch`](../arch/index.html) module.
//!
//! - The [`Target::Error`](trait.Target.html#associatedtype.Error) associated
//!   type allows implementors to plumb-through their own project-specific fatal
//!   error type into the `Target` trait. This is a big-boost to library
//!   ergonomics, as it enables consumers of `gdbstub` to preserve
//!   target-specific context while using `gdbstub`, without having to do any
//!   "error-stashing".
//!
//! For example: consider an emulated target where certain devices might return
//! a `MyEmuError::ContractViolation` error whenever they're accessed
//! "improperly" (e.g: setting registers in the wrong order). By setting `type
//! Error = MyEmuError`, the method signature of the `Target`'s `resume` method
//! becomes `fn resume(&mut self, ...) -> Result<_, MyEmuError>`, which makes it
//! possible to preserve the target-specific error while using `gdbstub`!
//!
//! ## Required Methods (Base Protocol)
//!
//! A minimal `Target` implementation only needs to implement a single method:
//! [`Target::base_ops`](trait.Target.html#tymethod.base_ops). This method is
//! used to select which set of [`base`](crate::target::ext::base)
//! debugging operations will be used to control the target. These are
//! fundamental operations such as reading/writing memory, etc...
//!
//! All other methods are entirely optional! Check out the
//! [`ext`](ext#modules) module for a full list of currently supported protocol
//! extensions.
//!
//! ## Optional Protocol Extensions
//!
//! The GDB protocol is _massive_, and there are plenty of optional protocol
//! extensions that targets can implement to enhance the base debugging
//! experience.
//!
//! These protocol extensions range from relatively mundane things such as
//! setting/removing breakpoints or reading/writing individual registers, but
//! also include fancy things such as support for time travel debugging, running
//! shell commands remotely, or even performing file IO on the target!
//!
//! `gdbstub` uses a somewhat unique approach to exposing these many features,
//! called **Inlinable Dyn Extension Traits (IDETs)**. While this might sound a
//! bit daunting, the API is actually quite straightforward, and described in
//! great detail under the [`ext` module's documentation](ext).
//!
//! After getting the base protocol up and running, do take a moment to skim
//! through and familiarize yourself with the [many different protocol
//! extensions](ext# modules) that `gdbstub` implements. There are some really
//! nifty ones that you might not even realize you need!
//!
//! As a suggestion on where to start, consider implementing some of the
//! breakpoint related extensions under
//! [`breakpoints`](crate::target::ext::breakpoints). While setting/removing
//! breakpoints is technically an "optional" part of the GDB protocol, I'm sure
//! you'd be hard pressed to find a debugger that doesn't support breakpoints.
//!
//! ### Note: Missing Protocol Extensions
//!
//! `gdbstub`'s development is guided by the needs of its contributors, with
//! new features being added on an "as-needed" basis.
//!
//! If there's a GDB protocol extensions you're interested in that hasn't been
//! implemented in `gdbstub` yet, (e.g: remote filesystem access, tracepoint
//! support, etc...), consider opening an issue / filing a PR on the
//! [`gdbstub` GitHub repo](https://github.com/daniel5151/gdbstub/).
//!
//! Check out the [GDB Remote Configuration Docs](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Configuration.html)
//! for a table of GDB commands + their corresponding Remote Serial Protocol
//! packets.
//!
//! ### Example: A fairly minimal Single Threaded `Target`
//!
//! This example includes a handful of required and optional target features,
//! and shows off the basics of how to work with IDETs.
//!
//! ```rust
//! use gdbstub::common::Signal;
//! use gdbstub::target::{Target, TargetResult};
//! use gdbstub::target::ext::base::BaseOps;
//! use gdbstub::target::ext::base::singlethread::{
//!     SingleThreadResumeOps, SingleThreadSingleStepOps
//! };
//! use gdbstub::target::ext::base::singlethread::{
//!     SingleThreadBase, SingleThreadResume, SingleThreadSingleStep
//! };
//! use gdbstub::target::ext::breakpoints::{Breakpoints, SwBreakpoint};
//! use gdbstub::target::ext::breakpoints::{BreakpointsOps, SwBreakpointOps};
//!
//! struct MyTarget;
//!
//! impl Target for MyTarget {
//!     type Error = ();
//!     type Arch = gdbstub_arch::arm::Armv4t; // as an example
//!
//!     #[inline(always)]
//!     fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
//!         BaseOps::SingleThread(self)
//!     }
//!
//!     // opt-in to support for setting/removing breakpoints
//!     #[inline(always)]
//!     fn support_breakpoints(&mut self) -> Option<BreakpointsOps<Self>> {
//!         Some(self)
//!     }
//! }
//!
//! impl SingleThreadBase for MyTarget {
//!     fn read_registers(
//!         &mut self,
//!         regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
//!     ) -> TargetResult<(), Self> { todo!() }
//!
//!     fn write_registers(
//!         &mut self,
//!         regs: &gdbstub_arch::arm::reg::ArmCoreRegs
//!     ) -> TargetResult<(), Self> { todo!() }
//!
//!     fn read_addrs(
//!         &mut self,
//!         start_addr: u32,
//!         data: &mut [u8],
//!     ) -> TargetResult<(), Self> { todo!() }
//!
//!     fn write_addrs(
//!         &mut self,
//!         start_addr: u32,
//!         data: &[u8],
//!     ) -> TargetResult<(), Self> { todo!() }
//!
//!     // most targets will want to support at resumption as well...
//!
//!     #[inline(always)]
//!     fn support_resume(&mut self) -> Option<SingleThreadResumeOps<Self>> {
//!         Some(self)
//!     }
//! }
//!
//! impl SingleThreadResume for MyTarget {
//!     fn resume(
//!         &mut self,
//!         signal: Option<Signal>,
//!     ) -> Result<(), Self::Error> { todo!() }
//!
//!     // ...and if the target supports resumption, it'll likely want to support
//!     // single-step resume as well
//!
//!     #[inline(always)]
//!     fn support_single_step(
//!         &mut self
//!     ) -> Option<SingleThreadSingleStepOps<'_, Self>> {
//!         Some(self)
//!     }
//! }
//!
//! impl SingleThreadSingleStep for MyTarget {
//!     fn step(
//!         &mut self,
//!         signal: Option<Signal>,
//!     ) -> Result<(), Self::Error> { todo!() }
//! }
//!
//! impl Breakpoints for MyTarget {
//!     // there are several kinds of breakpoints - this target uses software breakpoints
//!     #[inline(always)]
//!     fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<Self>> {
//!         Some(self)
//!     }
//! }
//!
//! impl SwBreakpoint for MyTarget {
//!     fn add_sw_breakpoint(
//!         &mut self,
//!         addr: u32,
//!         kind: gdbstub_arch::arm::ArmBreakpointKind,
//!     ) -> TargetResult<bool, Self> { todo!() }
//!
//!     fn remove_sw_breakpoint(
//!         &mut self,
//!         addr: u32,
//!         kind: gdbstub_arch::arm::ArmBreakpointKind,
//!     ) -> TargetResult<bool, Self> { todo!() }
//! }
//! ```
//!
//! ## A note on error handling
//!
//! As you explore the various protocol extension traits, you'll often find that
//! functions don't return a typical [`Result<T, Self::Error>`],
//! and will instead return a [`TargetResult<T, Self>`].
//!
//! At first glance this might look a bit strange, since it looks like the `Err`
//! variant of `TargetResult` is `Self` instead of `Self::Error`!
//!
//! Thankfully, there's a good reason for why that's the case. In a nutshell,
//! `TargetResult` wraps a typical `Result<T, Self::Error>` with a few
//! additional error types which can be reported back to the GDB client via the
//! GDB RSP.
//!
//! For example, if the GDB client tried to read memory from invalid memory,
//! instead of immediately terminating the entire debugging session, it's
//! possible to simply return a `Err(TargetError::Errno(14)) // EFAULT`, which
//! will notify the GDB client that the operation has failed.
//!
//! See the [`TargetError`] docs for more details.
//!
//! ## A note on all the `<Self::Arch as Arch>::` syntax
//!
//! As you explore `Target` and its many extension traits, you'll enounter
//! many method signatures that use this pretty gnarly bit of Rust type syntax.
//!
//! If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
//! gets fixed, then types like `<Self::Arch as Arch>::Foo` could be simplified
//! to just `Self::Arch::Foo`, but until then, the much more explicit
//! [fully qualified syntax](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#fully-qualified-syntax-for-disambiguation-calling-methods-with-the-same-name)
//! must be used instead.
//!
//! To improve the readability and maintainability of your own implementation,
//! it'd be best to swap out the fully qualified syntax with whatever concrete
//! type is being used. e.g: on a 32-bit target, instead of cluttering up a
//! method implementation with a parameter passed as `(addr: <Self::Arch as
//! Arch>::Usize)`, just write `(addr: u32)` directly.
use crate::arch::{Arch, SingleStepGdbBehavior};

pub mod ext;

/// The error type for various methods on `Target` and its assorted associated
/// extension traits.
///
/// # Error Handling over the GDB Remote Serial Protocol
///
/// The GDB Remote Serial Protocol has less-than-stellar support for error
/// handling, typically taking the form of a single-byte
/// [`errno`-style error codes](https://chromium.googlesource.com/chromiumos/docs/+/HEAD/constants/errnos.md).
/// Moreover, often times the GDB client will simply _ignore_ the specific error
/// code returned by the stub, and print a generic failure message instead.
///
/// As such, while it's certainly better to use appropriate error codes when
/// possible (e.g: returning a `EFAULT` (14) when reading from invalid memory),
/// it's often fine to simply return the more general `TargetError::NonFatal`
/// instead, and avoid the headache of picking a "descriptive" error code. Under
/// the good, `TargetError::NonFatal` is sent to the GDB client as a generic
/// `EREMOTEIO` (121) error.
///
/// # `From` and `Into` implementations
///
/// - `From<()>` -> `TargetError::NonFatal`
/// - `From<io::Error>` -> `TargetError::Io(io::Error)` (requires `std` feature)
///
/// When using a custom target-specific fatal error type, users are encouraged
/// to write the following impl to simplify error handling in `Target` methods:
///
/// ```rust
/// use gdbstub::target::TargetError;
///
/// /// Target-specific Fatal Error
/// enum MyTargetFatalError {
///     // ...
/// }
///
/// impl From<MyTargetFatalError> for TargetError<MyTargetFatalError> {
///     fn from(e: MyTargetFatalError) -> Self {
///         TargetError::Fatal(e)
///     }
/// }
/// ```
///
/// Unfortunately, a blanket impl such as `impl<T: Target> From<T::Error> for
/// TargetError<T::Error>` isn't possible, as it could result in impl conflicts.
/// For example, if a Target decided to use `()` as its fatal error type, then
/// there would be conflict with the existing `From<()>` impl.
#[non_exhaustive]
pub enum TargetError<E> {
    /// A non-specific, non-fatal error has occurred.
    NonFatal,
    /// Non-fatal I/O Error. Only available when the `std` feature is enabled.
    ///
    /// At the moment, this is just shorthand for
    /// `TargetError::NonFatal(e.raw_os_err().unwrap_or(121))`. Error code `121`
    /// corresponds to `EREMOTEIO`.
    ///
    /// In the future, `gdbstub` may add support for the "QEnableErrorStrings"
    /// LLDB protocol extension, which would allow sending additional error
    /// context (in the form of an ASCII string) when an I/O error occurs. If
    /// this is something you're interested in, consider opening a PR!
    #[cfg(feature = "std")]
    Io(std::io::Error),
    /// An operation-specific non-fatal error code.
    Errno(u8),
    /// A target-specific fatal error.
    ///
    /// **WARNING:** Returning this error will immediately terminate the GDB
    /// debugging session, and return a top-level `GdbStubError::TargetError`!
    Fatal(E),
}

/// Converts a `()` into a `TargetError::NonFatal`.
impl<E> From<()> for TargetError<E> {
    fn from(_: ()) -> TargetError<E> {
        TargetError::NonFatal
    }
}

/// Converts a `std::io::Error` into a `TargetError::Io`.
#[cfg(feature = "std")]
impl<E> From<std::io::Error> for TargetError<E> {
    fn from(e: std::io::Error) -> TargetError<E> {
        TargetError::Io(e)
    }
}

/// A specialized `Result` type for `Target` operations. Supports reporting
/// non-fatal errors back to the GDB client.
///
/// See [`TargetError`] for more details.
///
/// _Note:_ While it's typically parameterized as `TargetResult<T, Self>`, the
/// error value is in-fact `TargetError<Self::Error>` (not `Self`).
pub type TargetResult<T, Tgt> = Result<T, TargetError<<Tgt as Target>::Error>>;

/// Describes the architecture and capabilities of a target which can be
/// debugged by [`GdbStub`](../struct.GdbStub.html).
///
/// The [`Target`](trait.Target.html) trait describes how to control and modify
/// a system's execution state during a GDB debugging session, and serves as the
/// primary bridge between `gdbstub`'s generic protocol implementation and a
/// target's project/platform-specific code.
///
/// **`Target` is the most important trait in `gdbstub`, and must be implemented
/// by anyone who uses the library!**
///
/// Please refer to the the documentation in the [`target` module](self)
/// for more information on how to implement and work with `Target` and its
/// various extension traits.
pub trait Target {
    /// The target's architecture.
    type Arch: Arch;

    /// A target-specific **fatal** error.
    type Error;

    /// Base operations such as reading/writing from memory/registers,
    /// stopping/resuming the target, etc....
    ///
    /// For example, on a single-threaded target:
    ///
    /// ```rust
    /// use gdbstub::target::Target;
    /// use gdbstub::target::ext::base::BaseOps;
    /// use gdbstub::target::ext::base::singlethread::SingleThreadBase;
    /// # use gdbstub::target::TargetResult;
    /// # struct MyTarget;
    ///
    /// impl Target for MyTarget {
    ///     // ...
    ///     # type Arch = gdbstub_arch::arm::Armv4t;
    ///     # type Error = ();
    ///
    ///     fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
    ///         BaseOps::SingleThread(self)
    ///     }
    /// }
    ///
    /// // ...and then implement the associated base IDET
    /// impl SingleThreadBase for MyTarget {
    ///     // ...
    /// #   fn read_registers(
    /// #       &mut self,
    /// #       regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
    /// #   ) -> TargetResult<(), Self> { todo!() }
    /// #
    /// #   fn write_registers(
    /// #       &mut self,
    /// #       regs: &gdbstub_arch::arm::reg::ArmCoreRegs
    /// #   ) -> TargetResult<(), Self> { todo!() }
    /// #
    /// #   fn read_addrs(
    /// #       &mut self,
    /// #       start_addr: u32,
    /// #       data: &mut [u8],
    /// #   ) -> TargetResult<(), Self> { todo!() }
    /// #
    /// #   fn write_addrs(
    /// #       &mut self,
    /// #       start_addr: u32,
    /// #       data: &[u8],
    /// #   ) -> TargetResult<(), Self> { todo!() }
    /// }
    /// ```
    fn base_ops(&mut self) -> ext::base::BaseOps<'_, Self::Arch, Self::Error>;

    /// If the target supports resumption, but hasn't implemented explicit
    /// support for software breakpoints (via
    /// [`SwBreakpoints`](ext::breakpoints::SwBreakpoint)), notify the user
    /// that the GDB client may set "implicit" software breakpoints by
    /// rewriting the target's instruction stream.
    ///
    /// Targets that wish to use the GDB client's implicit software breakpoint
    /// handler must explicitly **opt-in** to this somewhat surprising GDB
    /// feature by overriding this method to return `true`.
    ///
    /// If you are reading these docs after having encountered a
    /// [`GdbStubError::ImplicitSwBreakpoints`] error, it's quite likely that
    /// you'll want to implement explicit support for software breakpoints.
    ///
    /// # Context
    ///
    /// An "implicit" software breakpoint is set by the GDB client by manually
    /// writing a software breakpoint instruction into target memory via the
    /// target's `write_addrs` implementation. i.e: the GDB client will
    /// overwrite the target's instruction stream with a software breakpoint
    /// instruction, with the expectation that the target has a implemented a
    /// breakpoint exception handler.
    ///
    /// # Implications
    ///
    /// While this is a reasonable (and useful!) bit of behavior when targeting
    /// many classes of remote stub (e.g: bare-metal, separate process), there
    /// are many `gdbstub` implementations that do _not_ implement "software
    /// breakpoints" by naively rewriting the target's instruction stream.
    ///
    /// - e.g: a `gdbstub` implemented in an emulator is unlikely to implement
    ///   "software breakpoints" by hooking into the emulated hardware's
    ///   breakpoint handler, and would likely implement "breakpoints" by
    ///   maintaining a list of addresses to stop at as part of its core
    ///   interpreter loop.
    /// - e.g: a `gdbstub` implemented in a hypervisor would require special
    ///   coordination with the guest kernel to support software breakpoints, as
    ///   there would need to be some way to distinguish between "in-guest"
    ///   debugging, and "hypervisor" debugging.
    ///
    /// As such, `gdbstub` includes this `guard_rail_implicit_sw_breakpoints`
    /// method.
    ///
    /// As the name suggests, this method acts as a "guard rail" that
    /// warns users from accidentally opting into this "implicit" breakpoint
    /// functionality, and being exceptionally confused as to why their
    /// target is acting weird.
    ///
    /// If `gdbstub` detects that the target has not implemented a software
    /// breakpoint handler, it will check if
    /// `guard_rail_implicit_sw_breakpoints()` has been enabled, and if it
    /// has not, it will trigger a runtime error that points the user at this
    /// very documentation.
    ///
    /// # A note on breakpoints
    ///
    /// Aside from setting breakpoints at the explicit behest of the user (e.g:
    /// when setting breakpoints via the `b` command in GDB), the GDB client may
    /// also set/remove _temporary breakpoints_ as part of other commands.
    ///
    /// e.g: On targets without native support for hardware single-stepping,
    /// calling `stepi` in GDB will result in the GDB client setting a temporary
    /// breakpoint on the next instruction + resuming via `continue` instead.
    ///
    /// [`GdbStubError::ImplicitSwBreakpoints`]:
    /// crate::stub::GdbStubError::ImplicitSwBreakpoints
    #[inline(always)]
    fn guard_rail_implicit_sw_breakpoints(&self) -> bool {
        false
    }

    /// Override the arch-level value for [`Arch::single_step_gdb_behavior`].
    ///
    /// If you are reading these docs after having encountered a
    /// [`GdbStubError::SingleStepGdbBehavior`] error, you may need to either:
    ///
    /// - implement support for single-step
    /// - disable existing support for single step
    /// - be a Good Citizen and perform a quick test to see what kind of
    ///   behavior your Arch exhibits.
    ///
    /// # WARNING
    ///
    /// Unless you _really_ know what you're doing (e.g: working on a dynamic
    /// target implementation, attempting to fix the underlying bug, etc...),
    /// you should **not** override this method, and instead follow the advice
    /// the error gives you.
    ///
    /// Incorrectly setting this method may lead to "unexpected packet" runtime
    /// errors!
    ///
    /// # Details
    ///
    /// This method provides an "escape hatch" for disabling a workaround for a
    /// bug in the mainline GDB client implementation.
    ///
    /// To squelch all errors, this method can be set to return
    /// [`SingleStepGdbBehavior::Optional`] (though as mentioned above - you
    /// should only do so if you're sure that's the right behavior).
    ///
    /// For more information, see the documentation for
    /// [`Arch::single_step_gdb_behavior`].
    ///
    /// [`GdbStubError::SingleStepGdbBehavior`]:
    /// crate::stub::GdbStubError::SingleStepGdbBehavior
    #[inline(always)]
    fn guard_rail_single_step_gdb_behavior(&self) -> SingleStepGdbBehavior {
        <Self::Arch as Arch>::single_step_gdb_behavior()
    }

    /// Enable/disable using the more efficient `X` packet to write to target
    /// memory (as opposed to the basic `M` packet).
    ///
    /// By default, this method returns `true`.
    ///
    /// _Author's note:_ Unless you're _really_ trying to squeeze `gdbstub` onto
    /// a particularly resource-constrained platform, you may as well leave this
    /// optimization enabled.
    #[inline(always)]
    fn use_x_upcase_packet(&self) -> bool {
        true
    }

    /// Whether `gdbstub` should provide a "stub" `resume` implementation on
    /// targets without support for resumption.
    ///
    /// At the time of writing, the mainline GDB client does not gracefully
    /// handle targets that do not support support resumption, and will hang
    /// indefinitely if a user inadvertently attempts to `continue` or `step`
    /// such a target.
    ///
    /// To make the `gdbstub` user experience a bit better, the library includes
    /// bit of "stub" code to gracefully handle these cases.
    ///
    /// If a user attempts to resume a target that hasn't implemented support
    /// for resumption, `gdbstub` will write a brief message back to the GDB
    /// client console, and will immediately return a "stopped with TRAP" stop
    /// reason.
    ///
    /// This method controls whether or not this bt of behavior is enabled.
    ///
    /// _Author's note:_ Unless you're _really_ trying to squeeze `gdbstub` onto
    /// a particularly resource-constrained platform, you may as well leave this
    /// enabled. The resulting stub code is entirely optimized out on targets
    /// that implement support for resumption.
    #[inline(always)]
    fn use_resume_stub(&self) -> bool {
        true
    }

    /// Enable/Disable the use of run-length encoding on outgoing packets.
    ///
    /// This is enabled by default, as RLE can save substantial amounts of
    /// bandwidth down the wire.
    ///
    /// _Author's note:_ There are essentially no reasons to disable RLE, unless
    /// you happen to be using a custom GDB client that doesn't support RLE.
    #[inline(always)]
    fn use_rle(&self) -> bool {
        true
    }

    /// Whether to send a target description XML to the client.
    ///
    /// Setting this to `false` will override both
    /// [`Target::support_target_description_xml_override`] and the associated
    /// [`Arch::target_description_xml`].
    ///
    /// _Author's note:_ Having the GDB client autodetect your target's
    /// architecture and register set is really useful, so unless you're
    /// _really_ trying to squeeze `gdbstub` onto a particularly
    /// resource-constrained platform, you may as well leave this enabled.
    #[inline(always)]
    fn use_target_description_xml(&self) -> bool {
        true
    }

    /// (LLDB extension) Whether to send register information to the client.
    ///
    /// Setting this to `false` will override both
    /// [`Target::support_register_info_override`] and the associated
    /// [`Arch::register_info`].
    ///
    /// _Author's note:_ Having the LLDB client autodetect your target's
    /// register set is really useful, so unless you're _really_ trying to
    /// squeeze `gdbstub` onto a particularly resource-constrained platform, you
    /// may as well leave this enabled.
    #[inline(always)]
    fn use_register_info(&self) -> bool {
        true
    }

    /// Support for setting / removing breakpoints.
    #[inline(always)]
    fn support_breakpoints(&mut self) -> Option<ext::breakpoints::BreakpointsOps<'_, Self>> {
        None
    }

    /// Support for handling custom GDB `monitor` commands.
    #[inline(always)]
    fn support_monitor_cmd(&mut self) -> Option<ext::monitor_cmd::MonitorCmdOps<'_, Self>> {
        None
    }

    /// Support for Extended Mode operations.
    #[inline(always)]
    fn support_extended_mode(&mut self) -> Option<ext::extended_mode::ExtendedModeOps<'_, Self>> {
        None
    }

    /// Support for handling requests to get the target's current section (or
    /// segment) offsets.
    #[inline(always)]
    fn support_section_offsets(
        &mut self,
    ) -> Option<ext::section_offsets::SectionOffsetsOps<'_, Self>> {
        None
    }

    /// Support for overriding the target description XML specified by
    /// `Target::Arch`.
    #[inline(always)]
    fn support_target_description_xml_override(
        &mut self,
    ) -> Option<ext::target_description_xml_override::TargetDescriptionXmlOverrideOps<'_, Self>>
    {
        None
    }

    /// (LLDB extension) Support for overriding the register info specified by
    /// `Target::Arch`.
    #[inline(always)]
    fn support_register_info_override(
        &mut self,
    ) -> Option<ext::register_info_override::RegisterInfoOverrideOps<'_, Self>> {
        None
    }

    /// Support for reading the target's memory map.
    #[inline(always)]
    fn support_memory_map(&mut self) -> Option<ext::memory_map::MemoryMapOps<'_, Self>> {
        None
    }

    /// Support for setting / removing syscall catchpoints.
    #[inline(always)]
    fn support_catch_syscalls(
        &mut self,
    ) -> Option<ext::catch_syscalls::CatchSyscallsOps<'_, Self>> {
        None
    }

    /// Support for Host I/O operations.
    #[inline(always)]
    fn support_host_io(&mut self) -> Option<ext::host_io::HostIoOps<'_, Self>> {
        None
    }

    /// Support for reading the current exec-file.
    #[inline(always)]
    fn support_exec_file(&mut self) -> Option<ext::exec_file::ExecFileOps<'_, Self>> {
        None
    }

    /// Support for reading the target's Auxillary Vector.
    #[inline(always)]
    fn support_auxv(&mut self) -> Option<ext::auxv::AuxvOps<'_, Self>> {
        None
    }
}

macro_rules! impl_dyn_target {
    ($type:ty) => {
        impl<A, E> Target for $type
        where
            A: Arch,
        {
            type Arch = A;
            type Error = E;

            fn base_ops(&mut self) -> ext::base::BaseOps<'_, Self::Arch, Self::Error> {
                (**self).base_ops()
            }

            fn guard_rail_implicit_sw_breakpoints(&self) -> bool {
                (**self).guard_rail_implicit_sw_breakpoints()
            }

            fn guard_rail_single_step_gdb_behavior(&self) -> SingleStepGdbBehavior {
                (**self).guard_rail_single_step_gdb_behavior()
            }

            fn use_x_upcase_packet(&self) -> bool {
                (**self).use_x_upcase_packet()
            }

            fn use_resume_stub(&self) -> bool {
                (**self).use_resume_stub()
            }

            fn use_rle(&self) -> bool {
                (**self).use_rle()
            }

            fn use_target_description_xml(&self) -> bool {
                (**self).use_target_description_xml()
            }

            fn use_register_info(&self) -> bool {
                (**self).use_register_info()
            }

            fn support_breakpoints(
                &mut self,
            ) -> Option<ext::breakpoints::BreakpointsOps<'_, Self>> {
                (**self).support_breakpoints()
            }

            fn support_monitor_cmd(&mut self) -> Option<ext::monitor_cmd::MonitorCmdOps<'_, Self>> {
                (**self).support_monitor_cmd()
            }

            fn support_extended_mode(
                &mut self,
            ) -> Option<ext::extended_mode::ExtendedModeOps<'_, Self>> {
                (**self).support_extended_mode()
            }

            fn support_section_offsets(
                &mut self,
            ) -> Option<ext::section_offsets::SectionOffsetsOps<'_, Self>> {
                (**self).support_section_offsets()
            }

            fn support_target_description_xml_override(
                &mut self,
            ) -> Option<
                ext::target_description_xml_override::TargetDescriptionXmlOverrideOps<'_, Self>,
            > {
                (**self).support_target_description_xml_override()
            }

            fn support_register_info_override(
                &mut self,
            ) -> Option<ext::register_info_override::RegisterInfoOverrideOps<'_, Self>> {
                (**self).support_register_info_override()
            }

            fn support_memory_map(&mut self) -> Option<ext::memory_map::MemoryMapOps<'_, Self>> {
                (**self).support_memory_map()
            }

            fn support_catch_syscalls(
                &mut self,
            ) -> Option<ext::catch_syscalls::CatchSyscallsOps<'_, Self>> {
                (**self).support_catch_syscalls()
            }

            fn support_host_io(&mut self) -> Option<ext::host_io::HostIoOps<'_, Self>> {
                (**self).support_host_io()
            }

            fn support_exec_file(&mut self) -> Option<ext::exec_file::ExecFileOps<'_, Self>> {
                (**self).support_exec_file()
            }

            fn support_auxv(&mut self) -> Option<ext::auxv::AuxvOps<'_, Self>> {
                (**self).support_auxv()
            }
        }
    };
}

impl_dyn_target!(&mut dyn Target<Arch = A, Error = E>);
#[cfg(feature = "alloc")]
impl_dyn_target!(alloc::boxed::Box<dyn Target<Arch = A, Error = E>>);
