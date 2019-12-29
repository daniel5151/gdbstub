use alloc::vec::Vec;

#[derive(PartialEq, Eq, Debug)]
pub struct vCont {
    pub actions: Vec<VContAction>,
}

impl vCont {
    pub fn parse(body: &str) -> Result<Self, ()> {
        let mut acts = body.split(';');
        acts.next();

        let mut actions = Vec::new();

        for s in acts {
            let mut s = s.split(':');
            let kind = s.next().ok_or(())?;
            // TODO: properly handle thread-id
            let _tid = s.next();

            actions.push(VContAction {
                kind: kind.parse()?,
                tid: None,
            })
        }

        Ok(vCont { actions })
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
    RangeStep(u64, u64), // FIXME: vCont 'r' should use Target::Usize
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
