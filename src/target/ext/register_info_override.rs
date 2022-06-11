//! (LLDB extension) Override the register info specified by `Target::Arch`.
use crate::arch::lldb::Register;
use crate::target::Target;

/// If your target is using the `qRegisterInfo` packet, this
/// token hast to be returned from the `register_info` function to guarantee
/// that the callback function to write the register info has been invoked.
pub struct CallbackToken<'a>(pub(crate) core::marker::PhantomData<&'a *mut ()>);

/// This struct is used internally by `gdbstub` to wrap a
/// callback function which has to be used to direct the `qRegisterInfo` query.
pub struct Callback<'a> {
    /// The callback function that is directing the `qRegisterInfo` query.
    pub(crate) cb: &'a mut dyn FnMut(Option<Register<'_>>),
    /// A token to guarantee the callback has been used.
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

/// Target Extension - Override the target register info
/// specified by `Target::Arch`.
///
/// _Note:_ Unless you're working with a particularly dynamic,
/// runtime-configurable target, it's unlikely that you'll need to implement
/// this extension.
pub trait RegisterInfoOverride: Target {
    /// Invoke `reg_info.write(reg)` where `reg` is a [`Register`
    /// ](crate::arch::lldb::Register) struct to write information of
    /// a single register or `reg_info.done()` if you want to end the
    /// `qRegisterInfo` packet exchange. These two methods will return a
    /// `CallbackToken`, which has to be returned from this method.
    fn register_info<'a>(
        &mut self,
        reg_id: usize,
        reg_info: Callback<'a>,
    ) -> Result<CallbackToken<'a>, Self::Error>;
}

define_ext!(RegisterInfoOverrideOps, RegisterInfoOverride);
