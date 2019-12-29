#[derive(PartialEq, Eq, Debug)]
pub struct vContQuestionMark;

impl vContQuestionMark {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(vContQuestionMark)
    }
}
