//! Extensions to [`Target`](super::Target) which add support for various
//! subsets of the GDB Remote Serial Protocol.
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
//! ## How Protocol Extensions Work - Inlineable Dyn Extension Traits (IDETs)
//!
//! The GDB protocol is massive, and contains all sorts of optional
//! functionality. In the early versions of `gdbstub`, the `Target` trait
//! directly implemented a method for _every single protocol extension_. If this
//! trend continued, there would've been literally _hundreds_ of associated
//! methods - of which only a small subset were ever used at once!
//!
//! Aside from the cognitive complexity of having so many methods on a single
//! trait, this approach had numerous other drawbacks as well:
//!
//!  - Implementations that did not implement all available protocol extensions
//!    still had to "pay" for the unused packet parsing/handler code, resulting
//!    in substantial code bloat, even on `no_std` platforms.
//!  - `GdbStub`'s internal implementation needed to include a large number of
//!    _runtime_ checks to deal with incorrectly implemented `Target`s.
//!      - No way to enforce "mutually-dependent" trait methods at compile-time.
//!          - e.g: When implementing hardware breakpoint extensions, targets
//!            _must_ implement both the `add_breakpoint` and
//!            `remove_breakpoints` methods.
//!      - No way to enforce "mutually-exclusive" trait methods at compile-time.
//!          - e.g: The `resume` method for single-threaded targets has a much
//!            simpler API than for multi-threaded targets, but it would be
//!            incorrect for a target to implement both.
//!
//! At first blush, it seems the the solution to all these issues is obvious:
//! simply tie each protocol extension to a `cargo` feature! And yes, while
//! this would indeed work, there would be several serious ergonomic drawbacks:
//!
//! - There would be _hundreds_ of individual feature flags that would need to
//!   be toggled by end users.
//! - It would be functionally impossible to _test_ all permutations of
//!   enabled/disabled cargo features.
//! - A single binary would need to rely on some [non-trivial `cargo`-fu](https://github.com/rust-lang/cargo/issues/674)
//!   in order to have multiple `Target` implementations in a single binary.
//!
//! After much experimentation and iteration, `gdbstub` ended up taking a
//! radically different approach to implementing and enumerating available
//! features, using a technique called **Inlineable Dyn Extension Traits**.
//!
//! > _Author's note:_ As far as I can tell, this isn't a very well-known trick,
//! or at the very least, I've personally never encountered any library that
//! uses this sort of API. As such, I've decided to be a bit cheeky and give it
//! a name! At some point, I'm hoping to write a standalone blog post which
//! further explores this technique, comparing it to other/existing approaches,
//! and diving into details of the how the compiler optimizes this sort of code.
//! In fact, I've already got a [very rough github repo](https://github.com/daniel5151/optional-trait-methods) with some of my
//! findings.
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
//! runtime-enumerable optional trait methods!
//!
//! #### Technical overview
//!
//! The basic principles behind Inlineable Dyn Extension Traits are best
//! explained though example:
//!
//! Lets say we want to add an optional protocol extension described by an
//! `ProtocolExt` trait to a base `Protocol` trait. How would we do that using
//! IDETs?
//!
//! - (library) Define a `trait ProtocolExt: Protocol { ... }` which includes
//!   all the methods required by the protocol extension:
//!    - _Note:_ Making `ProtocolExt` a subtrait of `Protocol` is not strictly
//!      required, but it does enable transparently using `Protocol`'s
//!      associated types as part of `ProtocolExt`'s method definitions.
//!
//! ```rust,ignore
//! /// `foo` and `bar` are mutually-dependent methods.
//! trait ProtocolExt: Protocol {
//!     fn foo(&self);
//!     // can use associated types in method signature!
//!     fn bar(&mut self) -> Result<(), Self::Error>;
//! }
//! ```
//!
//! - (library) "Associate" the `ProtocolExt` extension trait to the original
//!   `Protocol` trait by adding a new `Protocol` method that "downcasts" `self`
//!   into a `&mut dyn ProtocolExt`.
//!
//! ```rust,ignore
//! trait Protocol {
//!     // ... other methods ...
//!
//!     // Optional extension
//!     #[inline(always)]
//!     fn support_protocol_ext(&mut self) -> Option<ProtocolExtOps<Self>> {
//!         // disabled by default
//!         None
//!     }
//!
//!     // Mutually-exclusive extensions
//!     fn get_ext_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error>;
//! }
//!
//! // Using a typedef for readability
//! type ProtocolExtOps<T> =
//!     &'a mut dyn ProtocolExt<Arch = <T as Protocol>::Arch, Error = <T as Protocol>::Error>;
//!
//! enum EitherOrExt<A, E> {
//!     ProtocolExtA(&'a mut dyn ProtocolExtA<Arch = A, Error = E>),
//!     ProtocolExtB(&'a mut dyn ProtocolExtB<Arch = A, Error = E>),
//! }
//! ```
//!
//! - (user) Implements the `ProtocolExt` extension for their target (just like
//!   a normal trait).
//!
//! ```rust,ignore
//! impl ProtocolExt for MyTarget {
//!     fn foo(&self) { ... }
//!     fn bar(&mut self) -> Result<(), Self::Error> { ... }
//! }
//! ```
//!
//! - (user) Implements the base `Protocol` trait, overriding the
//!   `support_protocol_ext` method to return `Some(self)`, which will
//!   effectively "enable" the extension.
//!
//! ```rust,ignore
//! impl Protocol for MyTarget {
//!     // Optional extension
//!     #[inline(always)]
//!     fn support_protocol_ext(&mut self) -> Option<ProtocolExtOps<Self>> {
//!         Some(self) // will not compile unless `MyTarget` also implements `ProtocolExt`
//!     }
//!
//!     // Mutually-exclusive extensions
//!     #[inline(always)]
//!     fn get_ext_a_or_b(&mut self) -> EitherOrExt<Self::Arch, Self::Error> {
//!         EitherOrExt::ProtocolExtA(self)
//!     }
//! }
//! ```
//!
//! > Please note the use of `#[inline(always)]` when enabling IDET methods.
//! While LLVM is usually smart enough to inline single-level IDETs (such as in
//! the example above), nested IDETs will often require a bit of "help" from the
//! `inline` directive to be correctly optimized.
//!
//! Now, here's where IDETs really shine: If the user didn't implement
//! `ProtocolExt`, but _did_ try to enable the feature by overriding
//! `support_protocol_ext` to return `Some(self)`, they'll get a compile-time
//! error that looks something like this:
//!
//! ```text
//! error[E0277]: the trait bound `MyTarget: ProtocolExt` is not satisfied
//!   --> path/to/implementation.rs:44:14
//!    |
//! 44 |         Some(self)
//!    |              ^^^^ the trait `ProtocolExt` is not implemented for `MyTarget`
//!    |
//!    = note: required for the cast to the object type `dyn ProtocolExt<Arch = ..., Error = ...>`
//! ```
//!
//! The Rust compiler is preventing you from enabling a feature you haven't
//! implemented _at compile time!_
//!
//! - (library) Is able to _query_ whether or not an extension is available,
//!   _without_ having to actually invoke any method on the target!
//!
//! ```rust,ignore
//! fn execute_protocol(mut target: impl Target) {
//!     match target.support_protocol_ext() {
//!         Some(ops) => ops.foo(),
//!         None => { /* fallback when not enabled */ }
//!     }
//! }
//! ```
//!
//! This is already pretty cool, but what's _even cooler_ is that if you take a
//! look at the generated assembly of a monomorphized `execute_protocol` method
//! (e.g: using godbolt.org), you'll find that the compiler is able to
//! efficiently inline and devirtualize _all_ the calls to
//! `support_protocol_ext` method, which in-turn allows the dead-code-eliminator
//! to work its magic, and remove the unused branches from the generated code!
//! i.e: If a target implemention didn't implement the `ProtocolExt` extension,
//! then that `match` statement in `execute_protocol` would simply turn into a
//! noop!
//!
//! If IDETs are something you're interested in, consider checking out
//! [daniel5151/optional-trait-methods](https://github.com/daniel5151/optional-trait-methods)
//! for some sample code that shows off the power of IDETs. It's not
//! particularly polished, but it does includes code snippets which can be
//! pasted into godbolt.org directly to confirm the optimizations described
//! above, and a brief writeup which compares / contrasts alternatives to IDETs.
//!
//! Long story short: Optimizing compilers really are magic!
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
//!     - This is a really awesome trick: by wrapping code in an `if
//!       target.support_protocol_ext().is_some()` block, it's possible to
//!       specify _arbitrary_ blocks of code to be feature-dependent!
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

pub mod auxv;
pub mod base;
pub mod breakpoints;
pub mod catch_syscalls;
pub mod exec_file;
pub mod extended_mode;
pub mod host_io;
pub mod libraries;
pub mod lldb_register_info_override;
pub mod memory_map;
pub mod monitor_cmd;
pub mod section_offsets;
pub mod target_description_xml_override;
pub mod thread_extra_info;
