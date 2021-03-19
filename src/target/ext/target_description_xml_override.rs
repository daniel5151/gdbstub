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
    /// Refer to the
    /// [target_description_xml](crate::arch::Arch::target_description_xml)
    /// docs for more info.
    fn target_description_xml(&self) -> &str;
}

define_ext!(
    TargetDescriptionXmlOverrideOps,
    TargetDescriptionXmlOverride
);
