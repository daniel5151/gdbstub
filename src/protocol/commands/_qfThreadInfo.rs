#[derive(PartialEq, Eq, Debug)]
pub struct qfThreadInfo;

impl qfThreadInfo {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(qfThreadInfo)
    }
}
