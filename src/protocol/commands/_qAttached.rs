#[derive(PartialEq, Eq, Debug)]
pub struct qAttached {
    pub pid: Option<isize>,
}

impl qAttached {
    pub fn parse(body: &str) -> Result<Self, ()> {
        Ok(qAttached {
            pid: if body.is_empty() {
                None
            } else {
                Some(body.parse::<isize>().map_err(drop)?)
            },
        })
    }
}
