use super::prelude::*;

#[derive(PartialEq, Eq, Debug)]
pub enum Op {
    StepContinue,
    Other,
}

#[derive(PartialEq, Eq, Debug)]
pub struct H {
    pub kind: Op,
    pub thread: ThreadId,
}

impl<'a> ParseCommand<'a> for H {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        if body.is_empty() {
            return None;
        }

        let kind = match body.chars().next()? {
            'g' => Op::Other,
            'c' => Op::StepContinue,
            _ => return None,
        };
        let thread = body[1..].parse::<ThreadId>().ok()?;

        Some(H { kind, thread })
    }
}
