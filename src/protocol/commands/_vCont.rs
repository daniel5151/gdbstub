use core::convert::TryFrom;

use crate::protocol::common::Tid;

#[derive(Debug)]
pub struct vCont<'a> {
    pub actions: Actions<'a>,
}

impl<'a> TryFrom<&'a str> for vCont<'a> {
    type Error = ();

    fn try_from(body: &'a str) -> Result<Self, ()> {
        Ok(vCont {
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
            "C" => ContinueWithSig(u8::from_str_radix(s.next().ok_or(())?, 16).map_err(drop)?),
            "s" => Step,
            "S" => StepWithSig(u8::from_str_radix(s.next().ok_or(())?, 16).map_err(drop)?),
            "t" => Stop,
            "r" => {
                let mut range = s.next().ok_or(())?.split(',');
                let start = u64::from_str_radix(range.next().ok_or(())?, 16).map_err(drop)?;
                let end = u64::from_str_radix(range.next().ok_or(())?, 16).map_err(drop)?;
                RangeStep(start, end)
            }
            _ => return Err(()),
        };
        Ok(res)
    }
}
