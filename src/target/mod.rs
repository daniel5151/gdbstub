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
//! > _Aside:_ What's with all the `<Self::Arch as Arch>::` syntax?
//!
//! > As you explore `Target` and its many extension traits, you'll enounter
//! many method signatures that use this pretty gnarly bit of Rust type syntax.
//!
//! > If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
//! gets fixed, then types like `<Self::Arch as Arch>::Foo` could be simplified
//! to just `Self::Arch::Foo`, but until then, the much more explicit
//! [fully qualified syntax](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#fully-qualified-syntax-for-disambiguation-calling-methods-with-the-same-name)
//! must be used instead.
//!
//! > To improve the readability and maintainability of your own implementation,
//! it'd be best to swap out the fully qualified syntax with whatever concrete
//! type is being used. e.g: on a 32-bit target, instead of cluttering up a
//! method implementation with a parameter passed as `(addr: <Self::Arch as
//! Arch>::Usize)`, just write `(addr: u32)` directly.
//!
//! ## Required Methods (Base Protocol)
//!
//! A minimal `Target` implementation only needs to implement a single method:
//! [`Target::base_ops`](trait.Target.html#tymethod.base_ops). This method is
//! used to select which set of [`base`](crate::target::ext::base)
//! debugging operations will be used to control the target. These are
//! fundamental operations such as starting/stopping execution, reading/writing
//! memory, etc...
//!
//! All other methods are entirely optional! Check out the
//! [`ext`] module for a full list of currently supported protocol extensions.
//!
//! ### Example: A Bare-Minimum Single Threaded `Target`
//!
//! ```rust
//! use gdbstub::target::{Target, TargetResult};
//! use gdbstub::target::ext::base::BaseOps;
//! use gdbstub::target::ext::base::singlethread::SingleThreadOps;
//! use gdbstub::target::ext::base::singlethread::{ResumeAction, StopReason};
//!
//! struct MyTarget;
//!
//! impl Target for MyTarget {
//!     type Error = ();
//!     type Arch = gdbstub_arch::arm::Armv4t; // as an example
//!
//!     fn base_ops(&mut self) -> BaseOps<Self::Arch, Self::Error> {
//!         BaseOps::SingleThread(self)
//!     }
//! }
//!
//! impl SingleThreadOps for MyTarget {
//!     fn resume(
//!         &mut self,
//!         action: ResumeAction,
//!     ) -> Result<(), ()> { todo!() }
//!
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
//! }
//! ```
//!
//! ## Optional Methods (Protocol Extensions)
//!
//! The GDB protocol is _massive_, and there are plenty of optional protocol
//! extensions that targets can implement to enhance the base debugging
//! experience. These protocol extensions range from relatively mundane things
//! such as setting/removing breakpoints or reading/writing individual
//! registers, but also include fancy things such as  support for time travel
//! debugging, running shell commands remotely, or even performing file IO on
//! the target!
//!
//! As a starting point, consider implementing some of the breakpoint related
//! extensions under [`breakpoints`](crate::target::ext::breakpoints). While
//! setting/removing breakpoints is technically an "optional" part of the GDB
//! protocol, I'm sure you'd be hard pressed to find a debugger that doesn't
//! support breakpoints.
//!
//! Please make sure to read and understand [the documentation](ext) regarding
//! how IDETs work!
//!
//! ### Note: Missing Protocol Extensions
//!
//! `gdbstub`'s development is guided by the needs of its contributors, with
//! new features being added on an "as-needed" basis.
//!
//! If there's a GDB protocol extensions you're interested in that hasn't been
//! implemented in `gdbstub` yet, (e.g: remote filesystem access, tracepoint
//! support, etc...), consider opening an issue / filing a PR on GitHub!
//!
//! Check out the [GDB Remote Configuration Docs](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Configuration.html)
//! for a table of GDB commands + their corresponding Remote Serial Protocol
//! packets.
//!
//! ## A note on error handling
//!
//! As you explore the various protocol extension traits, you'll often find that
//! functions don't return a typical [`Result<T, Self::Error>`],
//! and will instead return a [`TargetResult<T, Self>`].
//!
//! At first glance, this might look a bit strange, since it might look as
//! though the `Err` variant of `TargetResult` is actually `Self` instead of
//! `Self::Error`! Thankfully, there's a good reason for why that's the case,
//! which you can read about as part of the [`TargetError`] documentation.
//!
//! In a nutshell, `TargetResult` wraps a typical `Result<T, Self::Error>` with
//! a few additional error types which can be reported back to the GDB client
//! via the GDB RSP. For example, if the GDB client tried to read memory from
//! invalid memory, instead of immediately terminating the entire debugging
//! session, it's possible to simply return a `Err(TargetError::Errno(14)) //
//! EFAULT`, which will notify the GDB client that the operation has failed.

use crate::arch::Arch;

pub mod ext;

/// The error type for various methods on `Target` and its assorted associated
/// extension traits.
///
/// # Error Handling over the GDB Remote Serial Protocol
///
/// The GDB Remote Serial Protocol has less-than-stellar support for error
/// handling, typically taking the form of a single-byte
/// [`errno`-style error codes](https://www-numi.fnal.gov/offline_software/srt_public_context/WebDocs/Errors/unix_system_errors.html).
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
/// ```rust,ignore
/// type MyTargetFatalError = ...; // Target-specific Fatal Error
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
    /// **WARNING:** Returning this error will immediately halt the target's
    /// execution and return a `GdbStubError::TargetError` from `GdbStub::run`!
    ///
    /// Note that the debugging session will will _not_ be terminated, and can
    /// be resumed by calling `GdbStub::run` after resolving the error and/or
    /// setting up a post-mortem debugging environment.
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
    /// ```rust,ignore
    /// use gdbstub::target::Target;
    /// use gdbstub::target::base::singlethread::SingleThreadOps;
    ///
    /// impl SingleThreadOps for MyTarget {
    ///     // ...
    /// }
    ///
    /// impl Target for MyTarget {
    ///     fn base_ops(&mut self) -> base::BaseOps<Self::Arch, Self::Error> {
    ///         base::BaseOps::SingleThread(self)
    ///     }
    /// }
    /// ```
    fn base_ops(&mut self) -> ext::base::BaseOps<Self::Arch, Self::Error>;

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

    /// Set/Remove software breakpoints.
    #[inline(always)]
    fn breakpoints(&mut self) -> Option<ext::breakpoints::BreakpointsOps<Self>> {
        None
    }

    /// Handle custom GDB `monitor` commands.
    #[inline(always)]
    fn monitor_cmd(&mut self) -> Option<ext::monitor_cmd::MonitorCmdOps<Self>> {
        None
    }

    /// Support for Extended Mode operations.
    #[inline(always)]
    fn extended_mode(&mut self) -> Option<ext::extended_mode::ExtendedModeOps<Self>> {
        None
    }

    /// Handle requests to get the target's current section (or segment)
    /// offsets.
    #[inline(always)]
    fn section_offsets(&mut self) -> Option<ext::section_offsets::SectionOffsetsOps<Self>> {
        None
    }

    /// Override the target description XML specified by `Target::Arch`.
    #[inline(always)]
    fn target_description_xml_override(
        &mut self,
    ) -> Option<ext::target_description_xml_override::TargetDescriptionXmlOverrideOps<Self>> {
        None
    }

    /// Provide a target memory map.
    #[inline(always)]
    fn memory_map(&mut self) -> Option<ext::memory_map::MemoryMapOps<Self>> {
        None
    }

    /// Set/Remove syscall catchpoints.
    #[inline(always)]
    fn catch_syscalls(&mut self) -> Option<ext::catch_syscalls::CatchSyscallsOps<Self>> {
        None
    }

    /// Support Host I/O operations.
    #[inline(always)]
    fn host_io(&mut self) -> Option<ext::host_io::HostIoOps<Self>> {
        None
    }

    /// Provide exec-file
    fn exec_file(&mut self) -> Option<ext::exec_file::ExecFileOps<Self>> {
        None
    }

    /// Provide auxv
    fn auxv(&mut self) -> Option<ext::auxv::AuxvOps<Self>> {
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

            #[inline(always)]
            fn base_ops(&mut self) -> ext::base::BaseOps<Self::Arch, Self::Error> {
                (**self).base_ops()
            }

            #[inline(always)]
            fn breakpoints(&mut self) -> Option<ext::breakpoints::BreakpointsOps<Self>> {
                (**self).breakpoints()
            }

            #[inline(always)]
            fn catch_syscalls(&mut self) -> Option<ext::catch_syscalls::CatchSyscallsOps<Self>> {
                (**self).catch_syscalls()
            }

            #[inline(always)]
            fn monitor_cmd(&mut self) -> Option<ext::monitor_cmd::MonitorCmdOps<Self>> {
                (**self).monitor_cmd()
            }

            #[inline(always)]
            fn exec_file(&mut self) -> Option<ext::exec_file::ExecFileOps<Self>> {
                (**self).exec_file()
            }

            #[inline(always)]
            fn extended_mode(&mut self) -> Option<ext::extended_mode::ExtendedModeOps<Self>> {
                (**self).extended_mode()
            }

            #[inline(always)]
            fn host_io(&mut self) -> Option<ext::host_io::HostIoOps<Self>> {
                (**self).host_io()
            }

            #[inline(always)]
            fn memory_map(&mut self) -> Option<ext::memory_map::MemoryMapOps<Self>> {
                (**self).memory_map()
            }

            #[inline(always)]
            fn auxv(&mut self) -> Option<ext::auxv::AuxvOps<Self>> {
                (**self).auxv()
            }

            #[inline(always)]
            fn section_offsets(&mut self) -> Option<ext::section_offsets::SectionOffsetsOps<Self>> {
                (**self).section_offsets()
            }

            #[inline(always)]
            fn target_description_xml_override(
                &mut self,
            ) -> Option<ext::target_description_xml_override::TargetDescriptionXmlOverrideOps<Self>>
            {
                (**self).target_description_xml_override()
            }
        }
    };
}

impl_dyn_target!(&mut dyn Target<Arch = A, Error = E>);
#[cfg(feature = "alloc")]
impl_dyn_target!(alloc::boxed::Box<dyn Target<Arch = A, Error = E>>);
