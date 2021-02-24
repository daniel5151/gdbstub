//! Extensions to [`Target`](super::Target) which add support for various
//! subsets of the GDB Remote Serial Protocol.
//!
//! On it's own, the [`Target`](super::Target) trait doesn't actually include
//! any methods to debug the target. Instead, `Target` uses a collection of
//! "Inlineable Dyn Extension Traits" (IDETs) to optionally implement various
//! subsets of the GDB protocol. For more details on IDETs, scroll down to the
//! [How Protocol Extensions Work - Inlineable Dyn Extension Traits
//! (IDETs)](#how-protocol-extensions-work---inlineable-dyn-extension-traits-idets)
//! section below.
//!
//! As a starting point, consider implementing some of the extensions under
//! [`breakpoints`]. For example, adding support for Software Breakpoints would
//! require implementing the
//! [`breakpoints::SwBreakpoint`](breakpoints::SwBreakpoint) extension, and
//! overriding the `Target::sw_breakpoint` method to return `Some(self)`.
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
//! Check out the [GDB Remote Configuration Docs](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Configuration.html)
//! for a table of GDB commands + their corresponding Remote Serial Protocol
//! packets.
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
//! functionality. In previous versions of `gdbstub`, the `Target` trait would
//! directly have a method for _every single protocol extension_, resulting in
//! literally _hundreds_ of associated methods!
//!
//! This approach had numerous drawbacks:
//!
//!  - Implementations that did not implement all available protocol extensions
//!    still had to "pay" for the unused packet parsing/handler code, resulting
//!    in substantial code bloat, even on `no_std` platforms.
//!  - Required the `GdbStub` implementation to include runtime checks to deal
//!    with incorrectly implemented `Target`s.
//!      - No way to enforce "mutually-dependent" trait methods at compile-time.
//!          - e.g: When implementing hardware breakpoint extensions, targets
//!            _must_ implement both the `add_breakpoint` and
//!            `remove_breakpoints` methods.
//!      - No way to enforce "mutually-exclusive" trait methods at compile-time.
//!          - e.g: The `resume` method for single-threaded targets has a much
//!            simpler API than for multi-threaded targets, but it would be
//!            incorrect for a target to implement both.
//!
//! Starting from version `0.4.0`, `gdbstub` is taking a new approach to
//! implementing and enumerating available Target features, using a technique
//! called **Inlineable Dyn Extension Traits**.
//!
//! _Author's note:_ As far as I can tell, this isn't a very well-known trick,
//! or at the very least, I've personally never encountered any library that
//! uses this sort of API. As such, I've decided to be a bit cheeky and give it
//! a name! At some point, I'm hoping to write a standalone blog post which
//! further explores this technique, comparing it to other/existing approaches,
//! and diving into details of the how the compiler optimizes this sort of code.
//!
//! So, what are "Inlineable Dyn Extension Traits"? Well, let's break it down:
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
//! Lets say we want to add an optional protocol extension described by an
//! `OptExt` trait to the `Target` trait. How would we do that using IDETs?
//!
//! - (library) Define a `trait OptExt: Target { ... }` with all the optional
//!   methods:
//!    - Making `OptExt` a supertrait of `Target` enables using `Target`'s
//!      associated types.
//!
//! ```rust,ignore
//! /// `foo` and `bar` are mutually-dependent methods.
//! trait OptExt: Target {
//!     fn foo(&self);
//!     // can use associated types in method signature!
//!     fn bar(&mut self) -> Result<(), Self::Error>;
//! }
//! ```
//!
//! - (library) "Tie" the `OptExt` extension trait to the original `Target`
//!   trait by adding a new `Target` method that simply returns `self` cast to a
//!   `&mut dyn OptExt`:
//!
//! ```rust,ignore
//! trait Target {
//!     // Optional extension
//!     fn ext_optfeat(&mut self) -> Option<OptExtOps<Self>> {
//!         // disabled by default
//!         None
//!     }
//!     // Mutually-exclusive extensions
//!     fn ext_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error>;
//! }
//!
//! // Using a typedef for readability
//! type OptExtOps<T> =
//!     &'a mut dyn OptExt<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
//!
//! enum EitherOrExt<A, E> {
//!     OptExtA(&'a mut dyn OptExtA<Arch = A, Error = E>),
//!     OptExtB(&'a mut dyn OptExtB<Arch = A, Error = E>),
//! }
//! ```
//!
//! - (user) Implements the `OptExt` extension for their target (just like a
//!   normal trait).
//!
//! ```rust,ignore
//! impl OptExt for Target {
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
//!     fn ext_optfeat(&mut self) -> Option<OptExtOps<Self>> {
//!         Some(self) // will not compile unless `MyTarget` also implements `OptExt`
//!     }
//!     // Mutually-exclusive extensions
//!     fn ext_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error> {
//!         EitherOrExt::OptExtA(self)
//!     }
//! }
//! ```
//!
//! If the user didn't implement `OptExt`, but tried to return `Some(self)`,
//! they'll get an error similar to:
//!
//! ```text
//! error[E0277]: the trait bound `MyTarget: OptExt` is not satisfied
//!   --> path/to/implementation.rs:44:14
//!    |
//! 44 |         Some(self)
//!    |              ^^^^ the trait `OptExt` is not implemented for `MyTarget`
//!    |
//!    = note: required for the cast to the object type `dyn OptExt<Arch = ..., Error = ...>`
//! ```
//!
//! - (library) Can now _query_ whether or not the extension is available,
//!   _without_ having to actually invoke any method on the target!
//! ```rust,ignore
//! // in a method that accepts `target: impl Target`
//! match target.ext_optfeat() {
//!     Some(ops) => ops.cool_feature(),
//!     None => { /* do nothing */ }
//! }
//! ```
//!
//! Moreover, if you take a look at the generated assembly (e.g: using
//! godbolt.org), you'll find that the compiler is able to efficiently inline
//! and devirtualize all the single-line `ext_` methods, which in-turn allows
//! the dead-code-eliminator to work it's magic, and remove the unused branches
//! from the generated code! i.e: If a target didn't implement the `OptExt`
//! extension, then that `match` statement would be converted into a noop!
//!
//! Check out [daniel5151/optional-trait-methods](https://github.com/daniel5151/optional-trait-methods)
//! for some sample code that shows off the power of IDETs. It includes code
//! snippets which can be pasted into godbolt.org directly to confirm the
//! optimizations described above.
//!
//! Optimizing compilers really are magic!
//!
//! #### Summary: The Benefits of IDETs
//!
//! IDETs solve the numerous issues and shortcomings that arise from the
//! traditional single trait + "optional" methods approach:
//!
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
//!       parsing / handler code for unimplemented protocol extensions.

macro_rules! doc_comment {
    ($x:expr, $($tt:tt)*) => {
        #[doc = $x]
        $($tt)*
    };
}

macro_rules! define_ext {
    ($extname:ident, $exttrait:ident) => {
        doc_comment! {
            concat!("See [`", stringify!($exttrait), "`](trait.", stringify!($exttrait), ".html)."),
            pub type $extname<'a, T> =
                &'a mut dyn $exttrait<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
        }
    };
}

pub mod agent;
pub mod base;
pub mod breakpoints;
pub mod extended_mode;
pub mod monitor_cmd;
pub mod section_offsets;
