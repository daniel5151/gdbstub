use gdbstub::target;
use gdbstub::target::ext::agent::BytecodeId;
use gdbstub::target::TargetResult;

use crate::emu::Emu;

impl target::ext::agent::Agent for Emu {
    fn enabled(&mut self, _enabled: bool) -> Result<(), Self::Error> {
        Ok(())
    }

    fn register_bytecode(&mut self, bytecode: &[u8]) -> TargetResult<BytecodeId, Self> {
        let agent = self.agent.as_mut().unwrap();

        agent.bytecode_id_counter += 1;
        let id = BytecodeId::new(agent.bytecode_id_counter).unwrap();
        agent.agent_bytecode.insert(id, bytecode.to_vec());
        log::warn!("Registered {:?}", id);
        Ok(id)
    }

    fn unregister_bytecode(&mut self, id: BytecodeId) -> TargetResult<(), Self> {
        let agent = self.agent.as_mut().unwrap();

        agent.agent_bytecode.remove(&id);
        log::warn!("Unregistered {:?}", id);
        Ok(())
    }

    fn evaluate(&mut self, id: BytecodeId) -> TargetResult<u32, Self> {
        log::error!("Evaluating {:?} - STUBBED, RETURNING 0", id);
        Ok(0)
    }
}
