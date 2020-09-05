//! [`Target`](trait.Target.html) and it's various optional extension traits.
//!
//! The [`Target`](trait.Target.html) trait describes how to control and modify
//! a system's execution state during a GDB debugging session, and serves as the
//! primary bridge between `gdbstub`'s generic protocol implementation and a
//! target's project/platform-specific code.
//!
//! # How to implement `Target`
//!
//! ## Associated Types
//!
//! The [`Target::Arch`](trait.Target.html#associatedtype.Arch) associated type
//! encodes information about the target's architecture, such as it's pointer
//! size, register layout, etc... `gdbstub` comes with several built-in
//! architecture definitions, which can be found under the
//! [`arch`](../arch/index.html) module.
//!
//! The [`Target::Error`](trait.Target.html#associatedtype.Error) associated
//! type allows implementors to plumb-through their own project-specific error
//! type into the `Target` trait. This is a big-boost to library ergonomics, as
//! it enables consumers of `gdbstb` to reuse existing error-handling logic
//! that's already implemented in their project!
//!
//! For example, consider an emulator where certain devices might return a
//! `MyEmuError::Unimplemented` error, and calling `cpu.cycle()` returns a
//! `Result<(), MyEmuError>`. By setting `type Error = MyEmuError`, the method
//! signature of the `Target`'s `resume` method becomes `fn resume(&mut self,
//! ...) -> Result<_, MyEmuError>`, which makes it possible to use the `?`
//! operator directly within the `Target` implementation!
//!
//! ## Handling Protocol Extensions
//!
//! The GDB protocol is massive, and contains all sorts of optional
//! functionality. If the target trait had a method for every single operation
//! and extension the protocol supported, there would be literally _hundreds_ of
//! associated methods!
//!
//! This approach has numerous drawbacks:
//!
//!  - Code-bloat from having to include hundreds of "stub" implementations for
//!    optional methods.
//!  - Requires the `GdbStub` implementation to include runtime checks that
//!    catch incorrectly implemented `Target`s.
//!      - No way to enforce "mutually-dependent" trait methods at compile-time
//!          - e.g: When implementing hardware breakpoint extensions, targets
//!            _must_ implement both the `add_breakpoint` and
//!            `remove_breakpoints` methods.
//!      - No way to enforce "mutually-exclusive" trait methods at compile-time
//!          - e.g: The `resume` method for single-threaded targets has a much
//!            simpler API than for multi-threaded targets, but it would be
//!            incorrect for a target to implement both.
//!
//! Versions of `gdbstub` prior to `0.3` actually used a variation of this
//! approach, albeit with some clever type-level tricks to work around some
//! of the ergonomic issues listed above. Of course, those workarounds weren't
//! perfect, and resulted in a clunky API that still required users to manually
//! enforce certain invariants.
//!
//! Starting from version `0.3`, `gdbstub` is taking a new approach to
//! implementing and enumerating available Target features, using a technique I
//! like to call **Inlineable Dyn Extension Traits**.
//!
//! ### Inlineable Dyn Extension Traits (IDETs)
//!
//! _Author's note:_ As far as I can tell, this isn't a very well-known trick,
//! or at the very least, I've never encountered a library which uses this sort
//! of API. At some point, I hope to write a standalone blog post which further
//! explores this technique, comparing it to other/existing approaches, and
//! diving into details of the how the compiler optimizes this sort of code.
//!
//! What are "Inlineable Dyn Extension Traits"? Well, lets break it down:
//!
//! - **Extension Traits** - A common [Rust convention](https://rust-lang.github.io/rfcs/0445-extension-trait-conventions.html#what-is-an-extension-trait)
//!   to extend the functionality of a Trait, _without_ modifying the original
//!   trait.
//! - **Dyn** - Alludes to the use of Dynamic Dispatch via [Trait Objects](https://doc.rust-lang.org/book/ch17-02-trait-objects.html).
//! - **Inlineable** - Alludes to the fact that this approach can be easily
//!   inlined, making it a truly zero-cost abstraction.
//!
//! In a nutshell, Inlineable Dyn Extension Traits (or IDETs) are an abuse of
//! the Rust trait system + modern compiler optimizations to emulate
//! compile-time optional trait methods!
//!
//! #### Technical overview
//!
//! The basic principles behind Inlineable Dyn Extension Traits are best
//! explained though example:
//!
//! - (library) Create a new `trait MyFeat: Target { ... }`.
//!    - Making `MyFeat` a supertrait of `Target` enables using `Target`'s
//!      associated types.
//! - (library) "Link" the `MyFeat` extension to the original `Target` trait
//!   though a new `Target` method. The signature varies depending on the kind
//!   of extension:
//!
//! ```rust,ignore
//! // Using a typedef for readability
//! type MyFeatExt<T> =
//!     &'a mut dyn MyFeat<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
//!
//! trait Target {
//!     // Required extension
//!     fn ext_my_feat(&mut self) -> MyFeatExt<Self>;
//!     // Optional extension
//!     fn ext_my_feat(&mut self) -> Option<MyFeatExt<Self>> {
//!         None
//!     }
//!     // Mutually-exclusive extensions
//!     fn either_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error>;
//!
//! }
//! enum EitherOrExt<A, E> {
//!     MyFeatA(&'a mut dyn MyFeatA<Arch = A, Error = E>),
//!     MyFeatB(&'a mut dyn MyFeatB<Arch = A, Error = E>),
//! }
//! ```
//!
//! - (user) Implements `MyFeat` for their target.
//! - (user) Implements `Target`, returning `self` in whenever a
//!   `MyFeatExt<Self>` is required.
//!
//! ```rust,ignore
//! impl Target for MyTarget {
//!     // Required extension
//!     fn ext_my_feat(&mut self) -> MyFeatExt<Self> {
//!         self
//!     }
//!     // Optional extension - Implemented
//!     fn ext_my_optfeat(&mut self) -> Option<MyFeatExt<Self>> {
//!         Some(self) // will not compile unless `MyTarget` also implements `MyFeat`
//!     }
//!     // Mutually-exclusive extensions
//!     fn either_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error> {
//!         EitherOrExt::MyFeatA(self)
//!     }
//! }
//! ```
//!
//! - (library) Can now query whether or not the extension is available,
//!   _without_ having to actually invoke any method on the target!
//!
//! ```rust,ignore
//! // in a method that accepts `target: impl Target`
//! match target.ext_my_optfeat() {
//!     Some(ops) => ops.cool_feature(),
//!     None => { /* report unsupported */ }
//! }
//! ```
//!
//! If you take a look at the generated assembly (e.g: using godbolt.org),
//! you'll find that the compiler is able to inline and devirtualize all the
//! `ext_` methods, which in-turn allows the dead-code-eliminator to work it's
//! magic, and remove all unused branches from the library code! i.e: If a
//! target didn't support `MyFeat`, then the `match` statement above would be
//! equivalent to calling `self.cool_feature()` directly!
//!
//! Check out [daniel5151/optional-trait-methods](https://github.com/daniel5151/optional-trait-methods)
//! for some sample code that shows off the power of IDETs. It includes code
//! snippets which can be pasted into godbolt.org directly to confirm the
//! optimizations described above.
//!
//! Optimizing compilers really are magic!
//!
//! #### Benefits of IDETs
//!
//! IDETs solve the numerous issues and shortcomings that arise from the
//! traditional single trait + "optional" methods approach:
//!
//! - Reduced code-bloat, as there are significantly fewer methods that require
//!   stubbed default implementations
//!    - Moreover, default implementations typically share the exact same
//!      function signature (i.e: `fn(&mut self) -> Option<&T> { None }`), which
//!      means an [optimizing compiler](http://llvm.org/docs/Passes.html#mergefunc-merge-functions)
//!      should be able to emit a single function for the identical default
//!      implementations (that is, if they're not entirely inlined / dead-code eliminated).
//! - Compile-time enforcement of mutually-dependent methods
//!    - By grouping mutually-dependent methods behind a single extension trait
//!      and marking them all as required methods, the Rust compiler is able to
//!      catch missing mutually-dependent methods at compile time, with no need
//!      for any runtime checks!
//! - Compile-time enforcement of mutually-exclusive methods
//!    - By grouping mutually-exclusive methods behind two extension traits, and
//!      wrapping those in an `enum`, the API is able to document
//!      mutually-exclusive functions _at the type-level_, in-turn enabling the
//!      library to omit any runtime checks!
//!    - _Note:_ Strictly speaking, this isn't really compile time
//!      "enforcement", as there's nothing stopping an "adversarial"
//!      implementation from implementing both sets of methods, and then
//!      "flipping" between the two at runtime.
//!
//! * * *
//!
//! Phew, with all that out of the way, let's finally get back to the topic at
//! hand: how to implement the `Target` trait:
//!
//! ## Required Methods
//!
//! The [`Target::base_ops`](trait.Target.html#tymethod.base_ops) method
//! describes the base debugging operations that must be implemented by any
//! target. These are things such as starting/stopping execution,
//! reading/writing memory, etc..
//!
//! Single threaded targets should implement the
//! [`base::SingleThread`](base/trait.SingleThread.html) trait, and return
//! `base::BaseOps::SingleThread(self)`, while Multithreaded targets should
//! implement [`base::MultiThread`](base/trait.MultiThread.html) trait, and
//! return `base::BaseOps::MultiThread(self)`.
//!
//! The only other required method is
//! [`Target::sw_breakpoint`](trait.Target.html#tymethod.sw_breakpoint), which
//! requires all targets to, at the very least, support setting/removing
//! software breakpoints via the
//! [`ext::breakpoint::SwBreakpoint`](ext/breakpoint/trait.SwBreakpoint.html)
//! extension.
//!
//! ## Optional Protocol Extensions
//!
//! Getting a basic debugging session up-and-running only requires implementing
//! the aforementioned required methods, but depending on a target's
//! capabilities, it may be possible to greatly enhance the debugging experience
//! by implementing various GDB protocol extensions.
//!
//! For example, while a bare-metal AVR microcontroller might not support
//! hardware-watchpoints, an AVR _emulator_ could certainly implement a
//! watchpoint mechanism! Wiring up the emulator's watchpoints to `gdbstub`
//! would then be as easy as implementing the [`ext::breakpoint::
//! HwWatchpoint`](ext/breakpoint/trait.HwWatchpoint.html) extension, and
//! overriding the `Target::hw_watchpoint` method to return `Some(self)`.
//!
//! After implementing the core `Target` interface, I'd encourage you to browse
//! through the various extensions that are available, as there are some pretty
//! nifty GDB features that many people don't know about!
//!
//! If there's a GDB feature that you need that isn't implemented yet, feel free
//! to open an issue / file a PR on Github!
//!
//! ### Aside: What's with all the `<Self::Arch as Arch>::` syntax?
//!
//! Many of the method signatures across the various `Target` extension
//! traits include some pretty gnarly type syntax.
//!
//! If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
//! gets fixed, then types like `<Self::Arch as Arch>::Foo` could be simplified
//! to just `Self::Arch::Foo`. Until then, the much more explicit
//! [fully qualified syntax](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#fully-qualified-syntax-for-disambiguation-calling-methods-with-the-same-name)
//! must be used instead.
//!
//! When implementing `Target`, it's recommended to use the concrete type in the
//! method signature. e.g: on a 32-bit platform, instead of cluttering up the
//! implementation with `<Self::Arch as Arch>::Usize`, just use `u32` directly.

use crate::arch::Arch;

pub mod base;
pub mod ext;

/// Describes a target which can be debugged by a
/// [`GdbStub`](struct.GdbStub.html).
///
/// Please see the `target` module documentation for details on how to implement
/// and work with the `Target` trait.
pub trait Target {
    /// The target's architecture.
    type Arch: Arch;

    /// A target-specific **fatal** error.
    type Error;

    /// Base operations required to debug any target, such as stopping/resuming
    /// the target, reading/writing from memory/registers, etc....
    ///
    /// For example, on a single-threaded target:
    ///
    /// ```rust,ignore
    /// fn base_ops(&mut self) -> base::BaseOps<'_, Self::Arch, Self::Error> {
    ///     base::BaseOps::SingleThread(self)
    /// }
    /// ```
    fn base_ops(&mut self) -> base::BaseOps<Self::Arch, Self::Error>;

    /// Set/Remote software breakpoints. This is a _mandatory_ extension, and
    /// _must_ be implemented by all targets.
    ///
    /// The implementation should simply return `self`:
    ///
    /// ```rust,ignore
    /// fn sw_breakpoint(&mut self) -> ext::SwBreakpointExt<Self> {
    ///     self
    /// }
    /// ```
    ///
    /// _Note:_ No, this is not an API oversight. This method is explicitly
    /// defined to be somewhat "boilerplate-y" to ensure that all consumers of
    /// the library are familiar with how ["Inlineable Dyn Trait
    /// Extensions"](../index.html#inlineable-dyn-extension-traits-idets)
    /// work.
    fn sw_breakpoint(&mut self) -> ext::SwBreakpointExt<Self>;

    /// Set/Remote hardware breakpoints.
    fn hw_breakpoint(&mut self) -> Option<ext::HwBreakpointExt<Self>> {
        None
    }

    /// Set/Remote hardware watchpoints.
    fn hw_watchpoint(&mut self) -> Option<ext::HwWatchpointExt<Self>> {
        None
    }

    /// Handle custom GDB `monitor` commands.
    fn monitor_cmd(&mut self) -> Option<ext::MonitorCmdExt<Self>> {
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

            fn base_ops(&mut self) -> base::BaseOps<Self::Arch, Self::Error> {
                (**self).base_ops()
            }

            fn sw_breakpoint(&mut self) -> ext::SwBreakpointExt<Self> {
                (**self).sw_breakpoint()
            }

            fn hw_breakpoint(&mut self) -> Option<ext::HwBreakpointExt<Self>> {
                (**self).hw_breakpoint()
            }

            fn hw_watchpoint(&mut self) -> Option<ext::HwWatchpointExt<Self>> {
                (**self).hw_watchpoint()
            }

            fn monitor_cmd(&mut self) -> Option<ext::MonitorCmdExt<Self>> {
                (**self).monitor_cmd()
            }
        }
    };
}

impl_dyn_target!(&mut dyn Target<Arch = A, Error = E>);
#[cfg(feature = "alloc")]
impl_dyn_target!(alloc::boxed::Box<dyn Target<Arch = A, Error = E>>);
