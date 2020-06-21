use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct QuestionMark;

impl TryFrom<&str> for QuestionMark {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(QuestionMark)
    }
}
