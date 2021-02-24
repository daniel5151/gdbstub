use gdbstub::target;
use gdbstub::target::ext::agent::BytecodeId;
use gdbstub::target::TargetResult;

use gdbstub::arch::arm::reg::id::ArmCoreRegId;
use gdbstub::arch::RegId;

use crate::emu::Emu;

impl target::ext::agent::Agent for Emu {
    fn enabled(&mut self, _enabled: bool) -> Result<(), Self::Error> {
        Ok(())
    }

    fn register_bytecode(&mut self, bytecode: &[u8]) -> TargetResult<BytecodeId, Self> {
        self.agent.bytecode_id_counter += 1;
        let id = BytecodeId::new(self.agent.bytecode_id_counter).unwrap();
        self.agent.agent_bytecode.insert(id, bytecode.to_vec());
        log::warn!("Registered {:?}", id);
        Ok(id)
    }

    fn unregister_bytecode(&mut self, id: BytecodeId) -> TargetResult<(), Self> {
        self.agent.agent_bytecode.remove(&id);
        log::warn!("Unregistered {:?}", id);
        Ok(())
    }

    fn evaluate(&mut self, id: BytecodeId) -> TargetResult<u32, Self> {
        log::warn!("Executing {:?}", id);

        // FIXME: this clone is bad, and the API should be re-written to avoid this.
        // e.g: by decoupling the lifetime of the agent from the target.
        let code = self.agent.agent_bytecode.get(&id).unwrap();

        let mut result = gdb_agent::evaluate(&code).unwrap();
        let res = loop {
            match result {
                gdb_agent::AgentExpressionResult::Complete(value) => break value.0,
                gdb_agent::AgentExpressionResult::NeedsRegister {
                    register,
                    expression,
                } => {
                    let reg_id = ArmCoreRegId::from_raw_id(register as _).unwrap().0;
                    let reg = self
                        .cpu
                        .reg_get(self.cpu.mode(), super::cpu_reg_id(reg_id).unwrap());

                    result = expression
                        .resume_with_register(gdb_agent::Value(reg as _))
                        .unwrap()
                }
                gdb_agent::AgentExpressionResult::NeedsMemory {
                    address,
                    size,
                    expression,
                } => {
                    eprintln!("{}", size);
                    assert!(size <= 4);

                    let mut val: u32 = 0;
                    for addr in ((address.0 as u32)..).take(size as usize) {
                        use armv4t_emu::Memory;
                        val = (val << 8) | (self.mem.r8(addr) as u32);
                    }

                    result = expression
                        .resume_with_memory(gdb_agent::Value(val.to_be() as _))
                        .unwrap()
                }
            }
        };

        log::info!("bytecode expression result: {}", res);

        Ok(res as _)
    }
}
