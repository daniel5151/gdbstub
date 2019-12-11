/// 'g'
///
/// Read general registers.
///
/// Reply:
///
/// 'XX…'
/// Each byte of register data is described by two hex digits. The bytes with
/// the register are transmitted in target byte order. The size of each register
/// and their position within the 'g' packet are determined by the GDB internal
/// gdbarch functions DEPRECATED_REGISTER_RAW_SIZE and gdbarch_register_name.
///
/// When reading registers from a trace frame (see Using the Collected Data),
/// the stub may also return a string of literal 'x's in place of the register
/// data digits, to indicate that the corresponding register has not been
/// collected, thus its value is unavailable. For example, for an architecture
/// with 4 registers of 4 bytes each, the following reply indicates to GDB that
/// registers 0 and 2 have not been collected, while registers 1 and 3 have been
/// collected, and both have zero value:
///
/// -> g
/// <- xxxxxxxx00000000xxxxxxxx00000000
///
/// 'E NN'
/// for an error.
#[derive(PartialEq, Eq, Debug)]
pub struct g;

impl g {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(g)
    }
}
