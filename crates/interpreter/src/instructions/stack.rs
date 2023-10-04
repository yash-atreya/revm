use crate::{
    gas,
    primitives::{Spec, U256},
    Host, InstructionResult, Interpreter,
};

pub fn pop(interpreter: &mut Interpreter, _host: &mut dyn Host) {
    gas!(interpreter, gas::BASE);
    if let Err(result) = interpreter.stack.pop() {
        interpreter.instruction_result = result;
    }
}

/// EIP-3855: PUSH0 instruction
///
/// Introduce a new instruction which pushes the constant value 0 onto the stack.
pub fn push0<SPEC: Spec>(interpreter: &mut Interpreter, _host: &mut dyn Host) {
    check!(interpreter, SHANGHAI);
    gas!(interpreter, gas::BASE);
    if let Err(result) = interpreter.stack.push(U256::ZERO) {
        interpreter.instruction_result = result;
    }
}

pub fn push<const N: usize>(interpreter: &mut Interpreter, _host: &mut dyn Host) {
    gas!(interpreter, gas::VERYLOW);
    let start = interpreter.instruction_pointer;
    // Safety: In Analysis we appended needed bytes for bytecode so that we are safe to just add without
    // checking if it is out of bound. This makes both of our unsafes block safe to do.
    if let Err(result) = interpreter
        .stack
        .push_slice::<N>(unsafe { core::slice::from_raw_parts(start, N) })
    {
        interpreter.instruction_result = result;
        return;
    }
    interpreter.instruction_pointer = unsafe { start.add(N) };
}

pub fn dup<const N: usize>(interpreter: &mut Interpreter, _host: &mut dyn Host) {
    gas!(interpreter, gas::VERYLOW);
    if let Err(result) = interpreter.stack.dup::<N>() {
        interpreter.instruction_result = result;
    }
}

pub fn swap<const N: usize>(interpreter: &mut Interpreter, _host: &mut dyn Host) {
    gas!(interpreter, gas::VERYLOW);
    if let Err(result) = interpreter.stack.swap::<N>() {
        interpreter.instruction_result = result;
    }
}
