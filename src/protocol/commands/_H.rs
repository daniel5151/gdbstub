/// 'H op thread-id'
///
/// Set thread for subsequent operations ('m', 'M', 'g', 'G', et.al.). Depending
/// on the operation to be performed, op should be 'c' for step and continue
/// operations (note that this is deprecated, supporting the 'vCont' command is
/// a better option), and 'g' for other operations. The thread designator
/// thread-id has the format and interpretation described in thread-id syntax.
///
/// Reply:
///
/// 'OK'
/// for success
///
/// 'E NN'
/// for an error
#[derive(PartialEq, Eq, Debug)]
pub struct H {
    pub kind: char, // TODO: make this an enum
    pub id: isize,
}

impl H {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if body.is_empty() {
            return Err(());
        }

        let kind = body.chars().next().ok_or(())?;
        let id = body[1..].parse::<isize>().map_err(drop)?;

        Ok(H { kind, id })
    }
}
