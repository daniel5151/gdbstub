/// 'D'
/// 'D;pid'
///
/// The first form of the packet is used to detach GDB from the remote system.
/// It is sent to the remote target before GDB disconnects via the detach
/// command.
///
/// The second form, including a process ID, is used when multiprocess protocol
/// extensions are enabled (see multiprocess extensions), to detach only a
/// specific process. The pid is specified as a big-endian hex string.
///
/// Reply:
///
/// 'OK'
/// for success
///
/// 'E NN'
/// for an error
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
