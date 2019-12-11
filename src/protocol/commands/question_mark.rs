/// '?'
///
/// Indicate the reason the target halted. The reply is the same as for step and
/// continue. This packet has a special interpretation when the target is in
/// non-stop mode; see Remote Non-Stop.
///
/// https://sourceware.org/gdb/onlinedocs/gdb/Stop-Reply-Packets.html#Stop-Reply-Packets

#[derive(PartialEq, Eq, Debug)]
pub struct QuestionMark;

impl QuestionMark {
    pub fn parse(body: &str) -> Result<Self, ()> {
        if !body.is_empty() {
            return Err(());
        }
        Ok(QuestionMark)
    }
}
