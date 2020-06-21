use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct H {
    pub kind: char, // TODO: make this an enum
    pub id: isize,  // FIXME: 'H' has invlaid thread-id syntax
}

impl TryFrom<&str> for H {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        if body.is_empty() {
            return Err(());
        }

        let kind = body.chars().next().ok_or(())?;
        let id = body[1..].parse::<isize>().map_err(drop)?;

        Ok(H { kind, id })
    }
}
