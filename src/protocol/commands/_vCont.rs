use super::prelude::*;

#[derive(Debug)]
pub struct vCont<'a> {
    pub actions: Actions<'a>,
}

impl<'a> ParseCommand<'a> for vCont<'a> {
    fn from_packet(buf: PacketBuf<'a>) -> Option<Self> {
        let body = buf.into_body_str();
        Some(vCont {
            actions: Actions(body),
        })
    }
}

#[derive(Debug)]
pub struct Actions<'a>(&'a str);

impl<'a> Actions<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = Result<VContAction, &'static str>> + 'a {
        self.0.split(';').map(|act| {
            let mut s = act.split(':');
            let kind = s.next().ok_or("missing kind")?;
            let tid = match s.next() {
                Some(s) => Some(s.parse::<Tid>().map_err(|_| "invalid tid")?),
                None => None,
            };

            Ok(VContAction {
                kind: kind.parse().map_err(|_| "invalid kind")?,
                tid,
            })
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct VContAction {
    pub kind: VContKind,
    pub tid: Option<Tid>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum VContKind {
    Continue,
    ContinueWithSig(u8),
    RangeStep(u64, u64),
    Step,
    StepWithSig(u8),
    Stop,
}

impl core::str::FromStr for VContKind {
    type Err = ();

    fn from_str(s: &str) -> Result<VContKind, ()> {
        use self::VContKind::*;

        let mut s = s.split(' ');
        let res = match s.next().unwrap() {
            "c" => Continue,
            "C" => ContinueWithSig(decode_hex(s.next().ok_or(())?.as_bytes()).map_err(drop)?),
            "s" => Step,
            "S" => StepWithSig(decode_hex(s.next().ok_or(())?.as_bytes()).map_err(drop)?),
            "t" => Stop,
            "r" => {
                let mut range = s.next().ok_or(())?.split(',');
                let start = decode_hex(range.next().ok_or(())?.as_bytes()).map_err(drop)?;
                let end = decode_hex(range.next().ok_or(())?.as_bytes()).map_err(drop)?;
                RangeStep(start, end)
            }
            _ => return Err(()),
        };
        Ok(res)
    }
}
