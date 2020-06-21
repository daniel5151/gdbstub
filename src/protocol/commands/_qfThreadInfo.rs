use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct qfThreadInfo;

impl TryFrom<&str> for qfThreadInfo {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(qfThreadInfo)
    }
}
