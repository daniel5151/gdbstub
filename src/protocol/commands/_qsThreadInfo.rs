use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct qsThreadInfo;

impl TryFrom<&str> for qsThreadInfo {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(qsThreadInfo)
    }
}
