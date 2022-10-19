//! TODO

use crate::{conn::ConnectionExt, target::Target};

use super::{
    state_machine::GdbStubStateMachine, DisconnectReason, GdbStub, GdbStubError, IntoStopReason,
};

/// TODO
pub trait RunTarget: Target + Sized {
    /// TODO
    type StopReason: IntoStopReason<Self>;

    /// TODO
    fn step(&mut self) -> Result<Option<Self::StopReason>, <Self as Target>::Error>;

    /// TODO
    fn interrupt_received(&mut self) -> Result<Option<Self::StopReason>, <Self as Target>::Error>;

    /// TODO
    fn step_loop_count(&self) -> usize {
        1024
    }
}

/// TODO
pub fn run<C: ConnectionExt, T: RunTarget>(
    connection: C,
    target: &mut T,
) -> Result<DisconnectReason, GdbStubError<<T as Target>::Error, C::Error>> {
    let gdb = GdbStub::new(connection);
    let mut gdb = gdb.run_state_machine(target)?;
    loop {
        gdb = match gdb {
            GdbStubStateMachine::Idle(mut gdb) => {
                // needs more data, so perform a blocking read on the connection
                let byte = gdb
                    .borrow_conn()
                    .read()
                    .map_err(GdbStubError::ConnectionRead)?;
                gdb.incoming_data(target, byte)?
            }

            GdbStubStateMachine::Disconnected(gdb) => {
                // run_blocking keeps things simple, and doesn't expose a way to re-use the
                // state machine
                break Ok(gdb.get_reason());
            }

            GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                let maybe_stop_reason = target
                    .interrupt_received()
                    .map_err(GdbStubError::TargetError)?;
                gdb.interrupt_handled(target, maybe_stop_reason)?
            }

            GdbStubStateMachine::Running(mut gdb) => {
                let conn = gdb.borrow_conn();
                'outer: loop {
                    for _ in 0..target.step_loop_count() {
                        let maybe_stop_reason = target.step().map_err(GdbStubError::TargetError)?;
                        if let Some(stop_reason) = maybe_stop_reason {
                            break 'outer gdb.report_stop(target, stop_reason)?;
                        }
                    }
                    if conn.peek().map_err(GdbStubError::ConnectionRead)?.is_some() {
                        let data = conn.read().map_err(GdbStubError::ConnectionRead)?;
                        break gdb.incoming_data(target, data)?;
                    }
                }
            }
        }
    }
}
