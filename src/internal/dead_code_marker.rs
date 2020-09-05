/// NOTE: We don't have a proper black box in stable Rust. This is
/// a workaround implementation, that may have a too big performance overhead,
/// depending on operation, or it may fail to properly avoid having code
/// optimized out. It is good enough that it is used by default.
///
/// A function that is opaque to the optimizer, to allow benchmarks to
/// pretend to use outputs to assist in avoiding dead-code
/// elimination.
// copied from https://docs.rs/bencher/0.1.5/src/bencher/lib.rs.html#590-596
#[cfg(feature = "__dead_code_marker")]
pub fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = core::ptr::read_volatile(&dummy);
        core::mem::forget(dummy);
        ret
    }
}

/// If the block of code which contains this macro doesn't get dead code
/// eliminated, this macro ensures that the resulting binary contains a
/// easy-to-find static string with the format `"<$feature,$ctx>"`.
///
/// In `gdbstub`, this macro makes it easy to see if the Rust compiler was able
/// to dead-code-eliminate the packet parsing / handling code associated with
/// unimplemented protocol extensions.
///
/// e.g: if the target didn't implement the `MonitorCmd` extension, then running
/// the unix command `strings <finalbinary> | grep "<qRcmd,"` should return no
/// results.
#[doc(hidden)]
#[macro_export]
macro_rules! __dead_code_marker {
    ($feature:literal, $ctx:literal) => {
        #[cfg(feature = "__dead_code_marker")]
        crate::internal::dead_code_marker::black_box(concat!("<", $feature, ",", $ctx, ">"));
    };
}
