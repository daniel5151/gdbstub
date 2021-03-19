//! Override the target description XML specified by `Target::Arch`.
use crate::target::Target;

/// Target Extension - Override the target description XML specified by
/// `Target::Arch`.
///
/// _Note:_ Unless you're working with a particularly dynamic,
/// runtime-configurable target, it's unlikely that you'll need to implement
/// this extension.
pub trait TargetDescriptionXmlOverride: Target {
    /// Return the target's description XML file (`target.xml`).
    ///
    /// Implementing this method enables GDB to automatically detect the
    /// target's architecture, saving the hassle of having to run `set
    /// architecture <arch>` when starting a debugging session.
    ///
    /// These descriptions can be quite succinct. For example, the target
    /// description for an `armv4t` target can be as simple as:
    ///
    /// ```
    /// r#"<target version="1.0"><architecture>armv4t</architecture></target>"#;
    /// ```
    ///
    /// See the [GDB docs](https://sourceware.org/gdb/current/onlinedocs/gdb/Target-Description-Format.html)
    /// for details on the target description XML format.
    fn target_description_xml(&self) -> &str;
}

define_ext!(
    TargetDescriptionXmlOverrideOps,
    TargetDescriptionXmlOverride
);
