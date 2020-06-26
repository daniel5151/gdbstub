use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct k;

impl TryFrom<&str> for k {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(k)
    }
}
