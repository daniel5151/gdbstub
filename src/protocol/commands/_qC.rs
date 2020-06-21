use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct qC;

impl TryFrom<&str> for qC {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(qC)
    }
}
