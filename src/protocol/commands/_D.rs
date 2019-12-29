#[derive(PartialEq, Eq, Debug)]
pub struct D {
    pub pid: Option<isize>,
}

impl D {
    pub fn parse(body: &str) -> Result<Self, ()> {
        Ok(D {
            pid: if body.is_empty() {
                None
            } else {
                Some(body.parse::<isize>().map_err(drop)?)
            },
        })
    }
}
