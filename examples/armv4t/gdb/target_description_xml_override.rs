use gdbstub::target;

use crate::emu::Emu;

impl target::ext::target_description_xml_override::TargetDescriptionXmlOverride for Emu {
    fn target_description_xml(&self) -> &str {
        r#"<target version="1.0"><!-- custom override string --><architecture>armv4t</architecture></target>"#
    }
}
