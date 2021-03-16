use crate::arch::Arch;
use crate::target::Target;

/// Basic operation to return an XML-formatted target description string
/// to the GDB client.
pub trait TargetDescription: Target {
    /// Returns an optional XML description for the target to GDB.
    fn target_description_xml(&self) -> &'static str {
        <Self::Arch as Arch>::target_description_xml().unwrap()
    }
}

/// See [`TargetDescription`]
pub type TargetDescriptionOps<'a, T> =
    &'a mut dyn TargetDescription<Arch = <T as Target>::Arch, Error = <T as Target>::Error>;
