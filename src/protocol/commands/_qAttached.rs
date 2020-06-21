use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct qAttached {
    pub pid: Option<isize>,
}

impl TryFrom<&str> for qAttached {
    type Error = ();

    fn try_from(body: &str) -> Result<Self, ()> {
        Ok(qAttached {
            pid: if body.is_empty() {
                None
            } else {
                Some(body.parse::<isize>().map_err(drop)?)
            },
        })
    }
}
