/// 'qC'
/// Return the current thread ID.
///
/// Reply:
///
/// 'QC thread-id'
/// Where thread-id is a thread ID as documented in thread-id syntax.
///
/// '(anything else)'
/// Any other reply implies the old thread ID.
#[derive(PartialEq, Eq, Debug)]
pub struct qC;

impl qC {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(qC)
    }
}
