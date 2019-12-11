/// 'qAttached:pid'
///
/// Return an indication of whether the remote server attached to an existing
/// process or created a new process. When the multiprocess protocol extensions
/// are supported (see multiprocess extensions), pid is an integer in
/// hexadecimal format identifying the target process. Otherwise, GDB will omit
/// the pid field and the query packet will be simplified as 'qAttached'.
///
/// This query is used, for example, to know whether the remote process should
/// be detached or killed when a GDB session is ended with the quit command.
///
/// Reply:
///
/// '1'
/// The remote server attached to an existing process.
///
/// '0'
/// The remote server created a new process.
///
/// 'E NN'
/// A badly formed request or an error was encountered.
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
