use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct vContQuestionMark;

impl TryFrom<&str> for vContQuestionMark {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(vContQuestionMark)
    }
}
