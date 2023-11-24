//! (LLDB extension) Override the register info specified by `Target::Arch`.

use crate::arch::lldb::Register;
use crate::target::Target;

/// This type serves as a "proof of callback", ensuring that either
/// `reg_info.done()` or `reg_info.write()` have been called from within the
/// `register_info` function. The only way to obtain a valid instance of this
/// type is by invoking one of those two methods.
pub struct CallbackToken<'a>(pub(crate) core::marker::PhantomData<&'a *mut ()>);

/// `register_info` callbacks
pub struct Callback<'a> {
    pub(crate) cb: &'a mut dyn FnMut(Option<Register<'_>>),
    pub(crate) token: CallbackToken<'a>,
}

impl<'a> Callback<'a> {
    /// The `qRegisterInfo` query shall be concluded.
    #[inline(always)]
    pub fn done(self) -> CallbackToken<'a> {
        (self.cb)(None);
        self.token
    }

    /// Write the register info of a single register.
    #[inline(always)]
    pub fn write(self, reg: Register<'_>) -> CallbackToken<'a> {
        (self.cb)(Some(reg));
        self.token
    }
}

/// Target Extension - Override the target register info specified by
/// `Target::Arch`.
///
/// _Note:_ Unless you're working with a particularly dynamic,
/// runtime-configurable target, it's unlikely that you'll need to implement
/// this extension.
pub trait LldbRegisterInfoOverride: Target {
    /// Invoke `reg_info.write(reg)` where `reg` is a [`Register`] struct to
    /// write information of a single register or `reg_info.done()` if you want
    /// to end the `qRegisterInfo` packet exchange.
    fn lldb_register_info<'a>(
        &mut self,
        reg_id: usize,
        reg_info: Callback<'a>,
    ) -> Result<CallbackToken<'a>, Self::Error>;
}

define_ext!(LldbRegisterInfoOverrideOps, LldbRegisterInfoOverride);
