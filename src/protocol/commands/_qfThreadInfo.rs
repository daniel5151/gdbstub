/// 'qfThreadInfo'
/// 'qsThreadInfo'
///
/// Obtain a list of all active thread IDs from the target (OS). Since there may
/// be too many active threads to fit into one reply packet, this query works
/// iteratively: it may require more than one query/reply sequence to obtain the
/// entire list of threads. The first query of the sequence will be the
/// 'qfThreadInfo' query; subsequent queries in the sequence will be the
/// 'qsThreadInfo' query.
///
/// NOTE: This packet replaces the 'qL' query (see below).
///
/// Reply:
///
/// 'm thread-id'
/// A single thread ID
///
/// 'm thread-id,thread-id…'
/// a comma-separated list of thread IDs
///
/// 'l'
/// (lower case letter 'L') denotes end of list.
///
/// In response to each query, the target will reply with a list of one or more
/// thread IDs, separated by commas. GDB will respond to each reply with a
/// request for more thread ids (using the 'qs' form of the query), until the
/// target responds with 'l' (lower-case ell, for last). Refer to thread-id
/// syntax, for the format of the thread-id fields.
///
/// Note: GDB will send the qfThreadInfo query during the initial connection
/// with the remote target, and the very first thread ID mentioned in the reply
/// will be stopped by GDB in a subsequent message. Therefore, the stub should
/// ensure that the first thread ID in the qfThreadInfo reply is suitable for
/// being stopped by GDB.
#[derive(PartialEq, Eq, Debug)]
pub struct qfThreadInfo;

impl qfThreadInfo {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(qfThreadInfo)
    }
}
