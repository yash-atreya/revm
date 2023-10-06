use alloc::boxed::Box;
use revm_interpreter::{
    instructions::control,
    opcode::{BoxedInstruction, BoxedInstructionTable, Instruction, InstructionTable},
    primitives::{db::Database, Spec},
    Host, InstructionResult, Interpreter,
};

use crate::EVMImpl;

/// Wrap instruction that would call inspector.
pub fn inspector_instruction<'a, H: Host + 'a>(
    instruction: Instruction<H>,
) -> BoxedInstruction<'a, H> {
    let inspector_instruction = Box::new(move |interpreter: &mut Interpreter, host: &mut H| {
        // step
        let result = host.step(interpreter);
        if result != InstructionResult::Continue {
            return;
        }

        // execute instruction.
        instruction(interpreter, host);

        // step ends
        host.step_end(interpreter);
    });

    inspector_instruction
}

/// make inspector table
pub fn make_inspector_instruction_table<
    'a,
    T: 'a,
    SPEC: Spec + 'static,
    DB: Database,
    const INSPECT: bool,
>(
    table: InstructionTable<EVMImpl<'a, SPEC, DB, INSPECT, T>>,
) -> BoxedInstructionTable<'a, EVMImpl<'a, SPEC, DB, INSPECT, T>> {
    let mut inspector_table: BoxedInstructionTable<'a, EVMImpl<'a, SPEC, DB, INSPECT, T>> =
        core::array::from_fn(|_| inspector_instruction(control::not_found));
    for (i, instruction) in table.iter().enumerate() {
        inspector_table[i] = Box::new(*instruction);
    }
    for (i, instruction) in table.iter().enumerate().step_by(4) {
        inspector_table[i] = inspector_instruction(*instruction);
    }
    inspector_table
}
