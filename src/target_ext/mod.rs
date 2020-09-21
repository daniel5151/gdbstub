//! Extensions to [`Target`](../trait.Target.html) which add support for various
//! subsets of the GDB Remote Serial Protocol.
//!
//! On it's own, the [`Target`](../trait.Target.html) trait doesn't actually
//! include any methods to debug the target; it simply describes the
//! architecture and capabilities of a target. Instead, `Target` uses a
//! collection of "Inlineable Dyn Extension Traits" (IDETs) to optionally
//! implement various subsets of the GDB protocol. For more details on IDETs,
//! see the
//! [How Protocol Extensions Work - Inlineable Dyn Extension Traits
//! (IDETs)](#how-protocol-extensions-work---inlineable-dyn-extension-traits-idets)
//! section below.
//!
//! As a starting point, consider implementing some of the extensions under
//! [`breakpoints`](breakpoints/index.html). For example, adding support for
//! Software Breakpoints would require implementing the
//! [`breakpoints::SwBreakpoint`](breakpoints/trait.SwBreakpoint.html)
//! extension, and overriding the `Target::sw_breakpoint` method to return
//! `Some(self)`.
//!
//! ### Note: Missing Protocol Extensions
//!
//! `gdbstub`'s development is guided by the needs of it's contributors, with
//! new features being added on an "as-needed" basis.
//!
//! If there's a GDB feature you need that hasn't been implemented yet, (e.g:
//! remote filesystem access, tracepoint support, etc...), consider opening an
//! issue / filing a PR on Github!
//!
//! ### Note: What's with all the `<Self::Arch as Arch>::` syntax?
//!
//! Many of the method signatures across the `Target` extension traits include
//! some pretty gnarly type syntax.
//!
//! If [rust-lang/rust#38078](https://github.com/rust-lang/rust/issues/38078)
//! gets fixed, then types like `<Self::Arch as Arch>::Foo` could be simplified
//! to just `Self::Arch::Foo`. Until then, the much more explicit
//! [fully qualified syntax](https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#fully-qualified-syntax-for-disambiguation-calling-methods-with-the-same-name)
//! must be used instead.
//!
//! When you come across this syntax, it's highly recommended to use the
//! concrete type instead. e.g: on a 32-bit target, instead of cluttering up
//! the implementation with `<Self::Arch as Arch>::Usize`, just use `u32`
//! directly.
//!
//! ## How Protocol Extensions Work - Inlineable Dyn Extension Traits (IDETs)
//!
//! The GDB protocol is massive, and contains all sorts of optional
//! functionality. If the `Target` trait had a method for every single operation
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
//! Versions of `gdbstub` prior to `0.4` actually used a variation of this
//! approach, albeit with some clever type-level tricks to work around some
//! of the ergonomic issues listed above. Of course, those workarounds weren't
//! perfect, and resulted in a clunky API that still required users to manually
//! enforce certain invariants themselves.
//!
//! Starting from version `0.4`, `gdbstub` is taking a new approach to
//! implementing and enumerating available Target features, using a technique I
//! like to call **Inlineable Dyn Extension Traits**.
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
//! the Rust trait system + modern compiler optimizations to emulate zero-cost,
//! runtime-query-able optional trait methods!
//!
//! #### Technical overview
//!
//! The basic principles behind Inlineable Dyn Extension Traits are best
//! explained though example:
//!
//! - (library) Create a new `trait OptFeat: Target { ... }`.
//!    - Making `OptFeat` a supertrait of `Target` enables using `Target`'s
//!      associated types.
//!
//! ```rust,ignore
//! /// `foo` and `bar` are mutually-dependent methods.
//! trait OptFeat: Target {
//!     fn foo(&self);
//!     // can use associated types in method signature!
//!     fn bar(&mut self) -> Result<(), Self::Error>;
//! }
//! ```
//!
//! - (library) "Tie" the `OptFeat` extension to the original `Target` trait
//!   though a new `Target` method which simply returns `self` cast to a `&mut
//!   dyn OptFeat`. The signature varies depending on the kind of extension:
//!
//! ```rust,ignore
//! trait Target {
//!     // Optional extension - disabled by default
//!     fn ext_optfeat(&mut self) -> Option<OptFeatOps<Self>> {
//!         None
//!     }
//!     // Mutually-exclusive extensions
//!     fn ext_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error>;
//! }
//!
//! // Using a typedef for readability
//! type OptFeatOps<T> =
//!     &'a mut dyn OptFeat<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
//!
//! enum EitherOrExt<A, E> {
//!     OptFeatA(&'a mut dyn OptFeatA<Arch = A, Error = E>),
//!     OptFeatB(&'a mut dyn OptFeatB<Arch = A, Error = E>),
//! }
//! ```
//!
//! - (user) Implements the `OptFeat` extension for their target (just like a
//!   normal trait).
//!
//! ```rust,ignore
//! impl OptFeat for Target {
//!     fn foo(&self) { ... }
//!     fn bar(&mut self) -> Result<(), Self::Error> { ... }
//! }
//! ```
//!
//! - (user) Implements the base `Target` trait, returning `Some(self)` to
//!   "enable" an extension, or `None` to leave it disabled.
//!
//! ```rust,ignore
//! impl Target for MyTarget {
//!     // Optional extension - Always enabled
//!     fn ext_optfeat(&mut self) -> Option<OptFeatOps<Self>> {
//!         Some(self) // will not compile unless `MyTarget` also implements `OptFeat`
//!     }
//!     // Mutually-exclusive extensions
//!     fn ext_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error> {
//!         EitherOrExt::OptFeatA(self)
//!     }
//! }
//! ```
//!
//! - (library) Can now query whether or not the extension is available,
//!   _without_ having to actually invoke any method on the target!
//! ```rust,ignore
//! // in a method that accepts `target: impl Target`
//! match target.ext_optfeat() {
//!     Some(ops) => ops.cool_feature(),
//!     None => { /* report unsupported */ }
//! }
//! ```
//!
//! If you take a look at the generated assembly (e.g: using godbolt.org),
//! you'll find that the compiler is able to inline and devirtualize all the
//! single-line `ext_` methods, which in-turn allows the dead-code-eliminator to
//! work it's magic, and remove unused branches from the library code! i.e:
//! If a target didn't implement the `OptFeat` extension, then the `match`
//! statement above would be equivalent to calling `self.cool_feature()`
//! directly!
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
//! - **Reduced code-bloat**
//!   - There are significantly fewer methods that require stubbed default
//!     implementations.
//!    - Moreover, default implementations typically share the exact same
//!      function signature (i.e: `fn(&mut self) -> Option<&T> { None }`), which
//!      means an [optimizing compiler](http://llvm.org/docs/Passes.html#mergefunc-merge-functions)
//!      should be able to emit a single function for the identical default
//!      implementations (that is, if they're not entirely inlined / dead-code eliminated).
//! - **Compile-time enforcement of mutually-dependent methods**
//!    - By grouping mutually-dependent methods behind a single extension trait
//!      and marking them all as required methods, the Rust compiler is able to
//!      catch missing mutually-dependent methods at compile time, with no need
//!      for any runtime checks!
//! - **Compile-time enforcement of mutually-exclusive methods**
//!    - By grouping mutually-exclusive methods behind two extension traits, and
//!      wrapping those in an `enum`, the API is able to document
//!      mutually-exclusive functions _at the type-level_, in-turn enabling the
//!      library to omit any runtime checks!
//!    - _Note:_ Strictly speaking, this isn't really compile time
//!      "enforcement", as there's nothing stopping an "adversarial"
//!      implementation from implementing both sets of methods, and then
//!      "flipping" between the two at runtime. Nonetheless, it serves as a good
//!      guardrail.
//! - **Enforce dead-code-elimination _without_ `cargo` feature flags**
//!     - This is a really awesome trick: by wrapping code in a `if
//!       target.ext_optfeat().is_some()` block, it's possible to specify
//!       _arbitrary_ blocks of code to be feature-dependent!
//!     - This is used to great effect in `gdbstub` to optimize-out any packet
//!       parsing / handler code for unimplemented protocol extensions. `grep`
//!       for `__protocol_hint` in `gdbstub` to see an example of this in
//!       action!

/// Automatically derives various `From` implementation for `TargetError`
/// wrappers.
///
/// Requires the wrapper to include a `TargetError` variant.
macro_rules! target_error_wrapper {
    (
        $( #[$meta:meta] )* // captures attributes and docstring
        $pub:vis // (optional) pub, pub(crate), etc.
        enum $name:ident
        $($tt:tt)*
    ) => {
        $(#[$meta])*
        $pub enum $name $($tt)*

        #[cfg(feature = "std")]
        impl<E> From<std::io::Error> for $name<E> {
            fn from(e: std::io::Error) -> $name<E> {
                $name::TargetError(TargetError::Io(e))
            }
        }

        impl<E> From<()> for $name<E> {
            fn from(_: ()) -> $name<E> {
                $name::TargetError(TargetError::NonFatal)
            }
        }

        impl<E> From<TargetError<E>> for $name<E> {
            fn from(e: TargetError<E>) -> $name<E> {
                $name::TargetError(e)
            }
        }
    };
}

macro_rules! define_ext {
    ($extname:ident, $($exttrait:tt)+) => {
        #[allow(missing_docs)]
        pub type $extname<'a, T> =
            &'a mut dyn $($exttrait)+<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
    };
}

pub mod base;
pub mod breakpoints;
pub mod extended_mode;
pub mod monitor_cmd;
pub mod section_offsets;
