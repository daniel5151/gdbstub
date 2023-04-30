use crate::protocol::common::hex::decode_hex;
use crate::protocol::common::hex::decode_hex_buf;

// Breakpoint packets are split up like this:
//
// Z0,addr,kind[;cond_list…][;cmds:persist,cmd_list…]
//  \_________/
//       |
//     BasicBreakpoint
//  \_______________________________________________/
//                          |
//                  BytecodeBreakpoint
//
// If the target does not implement the `Agent` extension, only the
// `BasicBreakpoint` part is parsed, which helps cut down on binary bloat.

#[derive(Debug)]
pub struct BasicBreakpoint<'a> {
    pub type_: u8,
    pub addr: &'a [u8],
    /// architecture dependent
    pub kind: &'a [u8],
}

impl<'a> BasicBreakpoint<'a> {
    pub fn from_slice(body: &'a mut [u8]) -> Option<BasicBreakpoint<'a>> {
        let mut body = body.splitn_mut(4, |b| matches!(*b, b',' | b';'));
        let type_ = decode_hex(body.next()?).ok()?;
        let addr = decode_hex_buf(body.next()?).ok()?;
        let kind = decode_hex_buf(body.next()?).ok()?;

        Some(BasicBreakpoint { type_, addr, kind })
    }
}

#[derive(Debug)]
pub struct BytecodeBreakpoint<'a> {
    pub base: BasicBreakpoint<'a>,
    pub conds: Option<BytecodeList<'a>>,
    pub cmds_persist: Option<(BytecodeList<'a>, bool)>,
}

impl<'a> BytecodeBreakpoint<'a> {
    pub fn from_slice(body: &'a mut [u8]) -> Option<BytecodeBreakpoint<'a>> {
        let mut body = body.splitn_mut(2, |b| *b == b';');

        let base = BasicBreakpoint::from_slice(body.next()?)?;

        let mut conds = None;
        let mut cmds_persist = None;

        if let Some(rest) = body.next() {
            let mut s = rest.split_mut(|b| *b == b':');
            let (raw_conds, raw_cmds) = match (s.next(), s.next()) {
                (Some(a), Some(b)) => (Some(strip_suffix_mut(a, b";cmds")?), Some(b)),
                (Some(a), None) => {
                    if a.starts_with(b"cmds") {
                        (None, Some(a))
                    } else {
                        (Some(a), None)
                    }
                }
                _ => return None,
            };

            if let Some(raw_conds) = raw_conds {
                conds = Some(BytecodeList(raw_conds));
            }

            if let Some(raw_cmds) = raw_cmds {
                let mut raw_cmds = raw_cmds.split_mut(|b| *b == b',');
                let raw_persist = decode_hex::<u8>(raw_cmds.next()?).ok()? != 0;
                let raw_cmds = raw_cmds.next()?;

                cmds_persist = Some((BytecodeList(raw_cmds), raw_persist));
            }
        }

        Some(BytecodeBreakpoint {
            base,
            conds,
            cmds_persist,
        })
    }
}

fn strip_suffix_mut<'a, T>(slice: &'a mut [T], suffix: &[T]) -> Option<&'a mut [T]>
where
    T: PartialEq,
{
    let (len, n) = (slice.len(), suffix.len());
    if n <= len {
        let (head, tail) = slice.split_at_mut(len - n);
        if tail == suffix {
            return Some(head);
        }
    }
    None
}

/// A lazily evaluated iterator over a series of bytecode expressions.
#[derive(Debug)]
pub struct BytecodeList<'a>(&'a mut [u8]);

impl<'a> BytecodeList<'a> {
    #[allow(dead_code)]
    pub fn into_iter(self) -> impl Iterator<Item = Option<&'a [u8]>> + 'a {
        self.0.split_mut(|b| *b == b'X').skip(1).map(|s| {
            let mut s = s.split_mut(|b| *b == b',');
            let _len = s.next()?;
            let code = decode_hex_buf(s.next()?).ok()?;
            Some(code as &[u8])
        })
    }
}
