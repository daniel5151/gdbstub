/// 'qfThreadInfo'
///
/// See 'qfThreadInfo'
#[derive(PartialEq, Eq, Debug)]
pub struct qsThreadInfo;

impl qsThreadInfo {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(qsThreadInfo)
    }
}
