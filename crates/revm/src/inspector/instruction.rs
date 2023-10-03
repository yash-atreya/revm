use alloc::boxed::Box;
use alloc::sync::Arc;
use revm_interpreter::{
    instructions::control,
    opcode::{BoxedInstruction, BoxedInstructionTable, Instruction, InstructionTable},
    Host, InstructionResult, Interpreter,
};

/// Wrap instruction that would call inspector.
pub fn inspector_instruction(instruction: Instruction) -> BoxedInstruction {
    let inspector_instruction =
        Box::new(move |interpreter: &mut Interpreter, host: &mut dyn Host| {
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
pub fn make_inspector_instruction_table(table: InstructionTable) -> BoxedInstructionTable {
    let mut inspector_table: BoxedInstructionTable =
        core::array::from_fn(|_| inspector_instruction(control::not_found));
    for (i, instruction) in table.iter().enumerate() {
        inspector_table[i] = Box::new(*instruction);
    }
    for (i, instruction) in table.iter().enumerate().step_by(4) {
        inspector_table[i] = inspector_instruction(*instruction);
    }
    inspector_table
}
