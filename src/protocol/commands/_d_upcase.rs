use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct D {
    pub pid: Option<isize>,
}

impl TryFrom<&str> for D {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        Ok(D {
            pid: if body.is_empty() {
                None
            } else {
                Some(body.parse::<isize>().map_err(drop)?)
            },
        })
    }
}
