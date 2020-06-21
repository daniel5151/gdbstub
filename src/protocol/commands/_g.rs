use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct g;

impl TryFrom<&str> for g {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(g)
    }
}
