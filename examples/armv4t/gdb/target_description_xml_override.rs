use gdbstub::target;
use gdbstub::target::TargetError;
use gdbstub::target::TargetResult;

use super::copy_range_to_buf;
use crate::emu::Emu;

impl target::ext::target_description_xml_override::TargetDescriptionXmlOverride for Emu {
    fn target_description_xml(
        &self,
        annex: &[u8],
        offset: u64,
        length: usize,
        buf: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let xml = match annex {
            b"target.xml" => TARGET_XML.trim(),
            b"extra.xml" => EXTRA_XML.trim(),
            _ => return Err(TargetError::NonFatal),
        };

        Ok(copy_range_to_buf(
            xml.trim().as_bytes(),
            offset,
            length,
            buf,
        ))
    }
}

const TARGET_XML: &str = r#"
<?xml version="1.0"?>
<!DOCTYPE target SYSTEM "gdb-target.dtd">
<target version="1.0">
    <architecture>armv4t</architecture>
    <feature name="org.gnu.gdb.arm.core">
        <vector id="padding" type="uint32" count="25"/>

        <reg name="r0" bitsize="32" type="uint32"/>
        <reg name="r1" bitsize="32" type="uint32"/>
        <reg name="r2" bitsize="32" type="uint32"/>
        <reg name="r3" bitsize="32" type="uint32"/>
        <reg name="r4" bitsize="32" type="uint32"/>
        <reg name="r5" bitsize="32" type="uint32"/>
        <reg name="r6" bitsize="32" type="uint32"/>
        <reg name="r7" bitsize="32" type="uint32"/>
        <reg name="r8" bitsize="32" type="uint32"/>
        <reg name="r9" bitsize="32" type="uint32"/>
        <reg name="r10" bitsize="32" type="uint32"/>
        <reg name="r11" bitsize="32" type="uint32"/>
        <reg name="r12" bitsize="32" type="uint32"/>
        <reg name="sp" bitsize="32" type="data_ptr"/>
        <reg name="lr" bitsize="32"/>
        <reg name="pc" bitsize="32" type="code_ptr"/>

        <!--
            For some reason, my version of `gdb-multiarch` doesn't seem to
            respect "regnum", and will not parse this custom target.xml unless I
            manually include the padding bytes in the target description.

            On the bright side, AFAIK, there aren't all that many architectures
            that use padding bytes. Heck, the only reason armv4t uses padding is
            for historical reasons (see comment below).

            Odds are if you're defining your own custom arch, you won't run into
            this issue, since you can just lay out all the registers in the
            correct order.
        -->
        <reg name="padding" type="padding" bitsize="32"/>

        <!-- The CPSR is register 25, rather than register 16, because
        the FPA registers historically were placed between the PC
        and the CPSR in the "g" packet. -->
        <reg name="cpsr" bitsize="32" regnum="25"/>
    </feature>
    <xi:include href="extra.xml"/>
</target>
"#;

const EXTRA_XML: &str = r#"
<?xml version="1.0"?>
<!DOCTYPE target SYSTEM "gdb-target.dtd">
<feature name="custom-armv4t-extension">
    <!--
        maps to a simple scratch register within the emulator. the GDB
        client can read the register using `p $custom` and set it using
        `set $custom=1337`
    -->
    <reg name="custom" bitsize="32" type="uint32"/>

    <!--
        pseudo-register that return the current time when read.

        notably, i've set up the target to NOT send this register as part of
        the regular register list, which means that GDB will fetch/update
        this register via the 'p' and 'P' packets respectively
    -->
    <reg name="time" bitsize="32" type="uint32"/>
</feature>
"#;
