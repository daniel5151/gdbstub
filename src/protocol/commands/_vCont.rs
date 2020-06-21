use core::convert::TryFrom;

#[derive(PartialEq, Eq, Debug)]
pub struct vCont<'a> {
    i: usize,
    buf: &'a str,
}

impl<'a> TryFrom<&'a str> for vCont<'a> {
    type Error = ();

    fn try_from(body: &'a str) -> Result<Self, ()> {
        Ok(vCont { i: 0, buf: body })
    }
}

impl<'a> vCont<'a> {
    pub fn into_iter(self) -> impl Iterator<Item = Result<VContAction, &'static str>> + 'a {
        self.buf.split(';').map(|act| {
            let mut s = act.split(':');
            let kind = s.next().ok_or("missing kind")?;
            // TODO: properly handle thread-id
            let _tid = s.next();

            Ok(VContAction {
                kind: kind.parse().map_err(|_| "invalid kind")?,
                tid: None,
            })
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct VContAction {
    pub kind: VContKind,
    pub tid: Option<isize>, // FIXME: vCont has invalid thread-id syntax
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
