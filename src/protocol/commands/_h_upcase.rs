use super::prelude::*;
use crate::{common::Pid, protocol::common::thread_id::ThreadId};
use core::num::NonZeroUsize;
use core::option::Option;
#[derive(Debug)]
pub enum Op {
    StepContinue,
    Other,
}
#[derive(Debug)]
pub struct H {
    pub kind: Op,
    pub thread: ThreadId,
    pub process: Option<Pid>,
}
impl<'a> ParseCommand<'a> for H {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() {
            return None;
        }
        let kind = match body[0] {
            b'g' => Op::Other,
            b'c' => Op::StepContinue,
            _ => return None,
        };
        let thread: ThreadId;
        let process: Pid;
        if body[1] == b'p' { //process is attached via H
            let dot_index = body.iter().position(|&b| b == b'.')?;
            thread = body[dot_index+1..].try_into().ok()?;
            let hex_bytes = &body[2..dot_index];
            let mut result: usize = 0;
            for &byte in hex_bytes {
                result = result * 16 + match byte {
                    b'0'..=b'9' => (byte - b'0') as usize,
                    b'a'..=b'f' => (byte - b'a' + 10) as usize,
                    b'A'..=b'F' => (byte - b'A' + 10) as usize, 
                    _ => return Some(H { kind, thread, process: None }) // Return None if a non-hex character is found
                };
            }   
            if let Some(process) = NonZeroUsize::new(result).map(|non_zero| Pid::from(non_zero)) {
                Some(H { kind, thread, process: Some(process)})
            } else {
                Some(H { kind, thread, process: None})
            }
        } else {
            thread = body[1..].try_into().ok()?;
            Some(H { kind, thread, process: None})
        }
    }
}
