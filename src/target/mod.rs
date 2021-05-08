//! Everything related to the [`Target`] trait + associated extension traits.
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
//! `gdbstub` uses a technique called "Inlineable Dyn Extension Traits" (IDETs)
//! to expose an ergonomic and extensible interface to the GDB protocol. It's
//! not a very common pattern, and can seem a little "weird" at first glance,
//! but it's actually very straightforward to use!
//!
//! Please refer to the [documentation in the `ext` module](ext) for more
//! information on IDETs, and how they're used to implement `Target` and it's
//! various extension traits.
//!
//! **TL;DR:** Whenever you see a method that has `Option<FooOps>` in the return
//! type, that method should return `Some(self)` if the extension is
//! implemented, or `None` if it's unimplemented / disabled.
//!
//! ## Associated Types
//!
//! - The [`Target::Arch`](trait.Target.html#associatedtype.Arch) associated
//!   type encodes information about the target's architecture, such as it's
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
//! ## Required Methods
//!
//! The [`Target::base_ops`](trait.Target.html#tymethod.base_ops) method
//! describes the base debugging operations that must be implemented by any
//! target. These are things such as starting/stopping execution,
//! reading/writing memory, etc..
//!
//! All other methods are entirely optional! Check out the
//! [`ext`] module for a full list of currently supported protocol extensions.
//!
//! ## Example: A Bare-Minimum Single Threaded `Target`
//!
//! ```rust,ignore
//! use gdbstub::target::Target;
//! use gdbstub::target::ext::base::singlethread::SingleThreadOps;
//!
//! impl SingleThreadOps for MyTarget {
//!     // ... omitted for brevity
//! }
//!
//! impl Target for MyTarget {
//!     fn base_ops(&mut self) -> base::BaseOps<Self::Arch, Self::Error> {
//!         base::BaseOps::SingleThread(self)
//!     }
//! }
//! ```

use crate::arch::Arch;

pub mod ext;

/// The error type for various methods on `Target` and it's assorted associated
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
/// For example, if a Target decided to use `()` as it's fatal error type, then
/// there would be conflict with the existing `From<()>` impl.
#[non_exhaustive]
pub enum TargetError<E> {
    /// A non-specific, non-fatal error has occurred.
    NonFatal,
    /// I/O Error. Only available when the `std` feature is enabled.
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

/// A specialized `Result` type for `Target` operations.
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
/// for more information on how to implement and work with `Target` and it's
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
}

macro_rules! impl_dyn_target {
    ($type:ty) => {
        #[allow(clippy::type_complexity)]
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
            fn monitor_cmd(&mut self) -> Option<ext::monitor_cmd::MonitorCmdOps<Self>> {
                (**self).monitor_cmd()
            }

            #[inline(always)]
            fn extended_mode(&mut self) -> Option<ext::extended_mode::ExtendedModeOps<Self>> {
                (**self).extended_mode()
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
