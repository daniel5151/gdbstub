use super::prelude::*;
use crate::protocol::commands::ext::ExtendedMode;
use crate::target::ext::base::BaseOps;

use crate::FAKE_PID;

impl<T: Target, C: Connection> GdbStubImpl<T, C> {
    pub(crate) fn handle_extended_mode<'a>(
        &mut self,
        res: &mut ResponseWriter<C>,
        target: &mut T,
        command: ExtendedMode<'a>,
    ) -> Result<HandlerStatus, Error<T::Error, C::Error>> {
        let ops = match target.extended_mode() {
            Some(ops) => ops,
            None => return Ok(HandlerStatus::Handled),
        };

        let handler_status = match command {
            ExtendedMode::ExclamationMark(_cmd) => {
                ops.on_start().map_err(Error::TargetError)?;
                HandlerStatus::NeedsOK
            }
            ExtendedMode::R(_cmd) => {
                ops.restart().map_err(Error::TargetError)?;
                HandlerStatus::Handled
            }
            ExtendedMode::vAttach(cmd) => {
                ops.attach(cmd.pid).handle_error()?;

                #[cfg(feature = "alloc")]
                self.attached_pids.insert(cmd.pid, true);

                // TODO: sends OK when running in Non-Stop mode
                HandlerStatus::Handled
            }
            ExtendedMode::vRun(cmd) => {
                use crate::target::ext::extended_mode::Args;

                let mut pid = ops
                    .run(cmd.filename, Args::new(&mut cmd.args.into_iter()))
                    .handle_error()?;

                // on single-threaded systems, we'll ignore the provided PID and keep
                // using the FAKE_PID.
                if let BaseOps::SingleThread(_) = target.base_ops() {
                    pid = FAKE_PID;
                }

                let _ = pid; // squelch warning on no_std targets
                #[cfg(feature = "alloc")]
                self.attached_pids.insert(pid, false);

                // TODO: send a more descriptive stop packet?
                res.write_str("S05")?;
                HandlerStatus::Handled
            }
            // --------- ASLR --------- //
            ExtendedMode::QDisableRandomization(cmd) if ops.configure_aslr().is_some() => {
                let ops = ops.configure_aslr().unwrap();
                ops.cfg_aslr(cmd.value).handle_error()?;
                HandlerStatus::NeedsOK
            }
            // --------- Environment --------- //
            ExtendedMode::QEnvironmentHexEncoded(cmd) if ops.configure_env().is_some() => {
                let ops = ops.configure_env().unwrap();
                ops.set_env(cmd.key, cmd.value).handle_error()?;
                HandlerStatus::NeedsOK
            }
            ExtendedMode::QEnvironmentUnset(cmd) if ops.configure_env().is_some() => {
                let ops = ops.configure_env().unwrap();
                ops.remove_env(cmd.key).handle_error()?;
                HandlerStatus::NeedsOK
            }
            ExtendedMode::QEnvironmentReset(_cmd) if ops.configure_env().is_some() => {
                let ops = ops.configure_env().unwrap();
                ops.reset_env().handle_error()?;
                HandlerStatus::NeedsOK
            }
            // --------- Working Dir --------- //
            ExtendedMode::QSetWorkingDir(cmd) if ops.configure_working_dir().is_some() => {
                let ops = ops.configure_working_dir().unwrap();
                ops.cfg_working_dir(cmd.dir).handle_error()?;
                HandlerStatus::NeedsOK
            }
            // --------- Startup Shell --------- //
            ExtendedMode::QStartupWithShell(cmd) if ops.configure_startup_shell().is_some() => {
                let ops = ops.configure_startup_shell().unwrap();
                ops.cfg_startup_with_shell(cmd.value).handle_error()?;
                HandlerStatus::NeedsOK
            }
            _ => HandlerStatus::Handled,
        };

        Ok(handler_status)
    }
}
