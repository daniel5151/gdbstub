#[derive(PartialEq, Eq, Debug)]
pub struct QuestionMark;

impl QuestionMark {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(QuestionMark)
    }
}
