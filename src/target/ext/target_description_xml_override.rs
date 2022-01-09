//! Override the target description XML specified by `Target::Arch`.
use crate::target::{Target, TargetResult};

/// Target Extension - Override the target description XML specified by
/// `Target::Arch`.
///
/// _Note:_ Unless you're working with a particularly dynamic,
/// runtime-configurable target, it's unlikely that you'll need to implement
/// this extension.
pub trait TargetDescriptionXmlOverride: Target {
    /// Read a target's description XML file at the specified `annex`.
    ///
    /// The "root" `annex` will always be `b"target.xml"`, though advanced
    /// targets may choose to split `target.xml` into multiple files via the
    /// the `<xi:include href="other_file.xml"/>` XML tag. If the GDB client
    /// encounter any such tags, it will re-invoke this handler with `annex`
    /// specified to point to `b"other_file.xml"`.
    ///
    /// Refer to the
    /// [target_description_xml](crate::arch::Arch::target_description_xml)
    /// docs for more info.
    ///
    /// Return the number of bytes written into `buf` (which may be less than
    /// `length`).
    ///
    /// If `offset` is greater than the length of the underlying data, return
    /// `Ok(0)`.
    fn target_description_xml(
        &self,
        annex: &[u8],
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self>;
}

define_ext!(
    TargetDescriptionXmlOverrideOps,
    TargetDescriptionXmlOverride
);
