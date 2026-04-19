//! Procedural macros for gdbstub regression testing.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Attribute macro to wrap a test function with gdbstub regression testing boilerplate.
#[proc_macro_attribute]
pub fn gdbstub_test(args: TokenStream, input: TokenStream) -> TokenStream {
    let args_str = args.to_string().replace(" ", "");
    let mut parts = args_str.split(',');
    let target = parts.next().unwrap().to_string();
    let is_extended = parts.next() == Some("extended");

    let input = parse_macro_input!(input as ItemFn);
    
    let name = &input.sig.ident;
    let body = &input.block;
    let attrs = &input.attrs;
    let vis = &input.vis;

    let result = quote! {
        #[test]
        #(#attrs)*
        #vis fn #name() -> anyhow::Result<()> {
            use anyhow::Context;
            let target = #target;
            let test_elf = gdbstub_test::find_test_elf(target);
            if !test_elf.exists() {
                anyhow::bail!("Test ELF not found at {:?}. Did you build the examples?", test_elf);
            }

            let emu = gdbstub_test::EmulatorProcess::spawn(target).context("failed to spawn emulator")?;
            let mut gdb = gdbstub_test::GdbMiClient::spawn(None, test_elf.to_str().unwrap()).context("failed to spawn gdb")?;
            
            if #is_extended {
                gdb.connect_extended_uds(emu.uds_path().to_str().unwrap()).context("failed to connect gdb to emulator in extended mode")?;
            } else {
                gdb.connect_uds(emu.uds_path().to_str().unwrap()).context("failed to connect gdb to emulator")?;
            }
            
            let mut func = |mut gdb: gdbstub_test::GdbMiClient| -> anyhow::Result<()> {
                #body
            };
            
            func(gdb)
        }
    };

    result.into()
}
