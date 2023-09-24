pub mod mainnet;
#[cfg(feature = "optimism")]
pub mod optimism;

use revm_interpreter::primitives::db::Database;

use crate::interpreter::{Gas, InstructionResult};
use crate::primitives::{Env, Spec};
use crate::EVMData;

/// Handler acts as a proxy and allow to define different behavior for different
/// sections of the code. This allows nice integration of different chains.
pub struct Handler<DB: Database> {
    // Uses env, call resul and returned gas from the call to determine the gas
    // that is returned from transaction execution..
    pub call_return: fn(&Env, InstructionResult, Gas) -> Gas,
    pub reimburse_caller: fn(&mut EVMData<'_, DB>, &Gas, u64),
    pub reward_beneficiary: fn(&mut EVMData<'_, DB>, &Gas, u64),
    pub calculate_gas_refund: fn(&Env, &Gas) -> u64,
}

impl<DB: Database> Handler<DB> {
    /// Handler for the mainnet
    pub fn mainnet<SPEC: Spec>() -> Self {
        Self {
            call_return: mainnet::handle_call_return::<SPEC>,
            calculate_gas_refund: mainnet::calculate_gas_refund::<SPEC>,
            reimburse_caller: mainnet::handle_reimburse_caller::<SPEC, DB>,
            reward_beneficiary: mainnet::reward_beneficiary::<SPEC, DB>,
        }
    }

    /// Handler for the optimism
    #[cfg(feature = "optimism")]
    pub fn optimism<SPEC: Spec>() -> Self {
        Self {
            call_return: optimism::handle_call_return::<SPEC>,
            // we reinburse caller the same was as in mainnet.
            // Refund is calculated differently then mainnet.
            reimburse_caller: mainnet::handle_reimburse_caller::<SPEC, DB>,
            calculate_gas_refund: optimism::calculate_gas_refund::<SPEC>,
            reward_beneficiary: optimism::reward_beneficiary::<SPEC, DB>,
        }
    }

    /// Handle call return, depending on instruction result gas will be reimbursed or not.
    pub fn call_return(&self, env: &Env, call_result: InstructionResult, returned_gas: Gas) -> Gas {
        (self.call_return)(env, call_result, returned_gas)
    }

    /// Reimburse the caller with gas that were not spend.
    pub fn reimburse_caller(&self, data: &mut EVMData<'_, DB>, gas: &Gas, gas_refund: u64) {
        (self.reimburse_caller)(data, gas, gas_refund)
    }

    /// Calculate gas refund for transaction. Some chains have it disabled.
    pub fn calculate_gas_refund(&self, env: &Env, gas: &Gas) -> u64 {
        (self.calculate_gas_refund)(env, gas)
    }

    /// Reward beneficiary
    pub fn reward_beneficiary(&self, data: &mut EVMData<'_, DB>, gas: &Gas, gas_refund: u64) {
        (self.reward_beneficiary)(data, gas, gas_refund)
    }
}
