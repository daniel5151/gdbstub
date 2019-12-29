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
