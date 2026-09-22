use super::prelude::*;
use crate::protocol::common::thread_id::ThreadId;
use crate::protocol::ConcreteThreadId;

#[derive(Debug)]
pub struct qSBPFCallStack {
    pub tid: ConcreteThreadId,
}

impl<'a> ParseCommand<'a> for qSBPFCallStack {
    #[inline(always)]
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body();
        if body.is_empty() || body[0] != b':' {
            return None;
        }
        let tid = ConcreteThreadId::try_from(ThreadId::try_from(&body[1..]).ok()?).ok()?;
        Some(qSBPFCallStack { tid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &mut [u8]) -> Option<qSBPFCallStack> {
        let mut buf = PacketBuf::new_with_raw_body(body).unwrap();
        assert!(buf.strip_prefix(b"qSBPFCallStack"));
        qSBPFCallStack::from_packet(buf)
    }

    #[test]
    fn parses_concrete_thread() {
        let packet = parse(&mut *b"qSBPFCallStack:2a".to_vec()).unwrap();
        assert_eq!(packet.tid.pid, None);
        assert_eq!(packet.tid.tid.get(), 0x2a);
    }

    #[test]
    fn rejects_missing_or_non_concrete_thread() {
        assert!(parse(&mut *b"qSBPFCallStack".to_vec()).is_none());
        assert!(parse(&mut *b"qSBPFCallStack:0".to_vec()).is_none());
        assert!(parse(&mut *b"qSBPFCallStack:-1".to_vec()).is_none());
    }
}
