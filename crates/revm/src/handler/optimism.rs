//! Handler related to Optimism chain

use crate::interpreter::{return_ok, return_revert, Gas, InstructionResult};
use crate::primitives::{Env, Spec};

/// Handle output of the transaction
#[cfg(feature = "optimism")]
pub fn handle_call_return<SPEC: Spec>(
    env: &Env,
    call_result: InstructionResult,
    returned_gas: Gas,
) -> Gas {
    use crate::primitives::SpecId::REGOLITH;
    let is_deposit = env.tx.optimism.source_hash.is_some();
    let is_optimism = env.cfg.optimism;
    let tx_system = env.tx.optimism.is_system_transaction;
    let tx_gas_limit = env.tx.gas_limit;
    let is_regolith = SPEC::enabled(REGOLITH);
    // Spend the gas limit. Gas is reimbursed when the tx returns successfully.
    let mut gas = Gas::new(tx_gas_limit);
    gas.record_cost(tx_gas_limit);

    if crate::USE_GAS {
        match call_result {
            return_ok!() => {
                // On Optimism, deposit transactions report gas usage uniquely to other
                // transactions due to them being pre-paid on L1.
                //
                // Hardfork Behavior:
                // - Bedrock (success path):
                //   - Deposit transactions (non-system) report their gas limit as the usage.
                //     No refunds.
                //   - Deposit transactions (system) report 0 gas used. No refunds.
                //   - Regular transactions report gas usage as normal.
                // - Regolith (success path):
                //   - Deposit transactions (all) report their gas used as normal. Refunds
                //     enabled.
                //   - Regular transactions report their gas used as normal.
                if is_optimism && (!is_deposit || is_regolith) {
                    // For regular transactions prior to Regolith and all transactions after
                    // Regolith, gas is reported as normal.
                    gas.erase_cost(returned_gas.remaining());
                    gas.record_refund(returned_gas.refunded());
                } else if is_deposit && tx_system.unwrap_or(false) {
                    // System transactions were a special type of deposit transaction in
                    // the Bedrock hardfork that did not incur any gas costs.
                    gas.erase_cost(tx_gas_limit);
                }
            }
            return_revert!() => {
                // On Optimism, deposit transactions report gas usage uniquely to other
                // transactions due to them being pre-paid on L1.
                //
                // Hardfork Behavior:
                // - Bedrock (revert path):
                //   - Deposit transactions (all) report the gas limit as the amount of gas
                //     used on failure. No refunds.
                //   - Regular transactions receive a refund on remaining gas as normal.
                // - Regolith (revert path):
                //   - Deposit transactions (all) report the actual gas used as the amount of
                //     gas used on failure. Refunds on remaining gas enabled.
                //   - Regular transactions receive a refund on remaining gas as normal.
                if is_optimism && (!is_deposit || is_regolith) {
                    gas.erase_cost(returned_gas.remaining());
                }
            }
            _ => {}
        }
    }
    gas
}

#[inline]
pub fn calculate_gas_refund(&self, gas: &Gas) -> u64 {
    let is_deposit = self.data.env.cfg.optimism && self.data.env.tx.optimism.source_hash.is_some();

    // Prior to Regolith, deposit transactions did not receive gas refunds.
    let is_gas_refund_disabled =
        (self.data.env.cfg.optimism && is_deposit && !SPEC::enabled(SpecId::REGOLITH));
    if is_gas_refund_disabled || self.data.env.cfg.is_gas_refund_disabled() {
        0
    } else {
        // EIP-3529: Reduction in refunds
        let max_refund_quotient = if GSPEC::enabled(LONDON) { 5 } else { 2 };
        min(gas.refunded() as u64, gas.spend() / max_refund_quotient)
    }
}

/// Reward beneficiary with gas fee.
#[inline]
pub fn reward_beneficiary<SPEC: Spec, DB: Database>(data: &mut EVMData<'_, DB>, gas: &Gas) {
    let is_deposit = self.data.env.cfg.optimism && self.data.env.tx.optimism.source_hash.is_some();
    let disable_coinbase_tip = (self.data.env.cfg.optimism && is_deposit);

    // transfer fee to coinbase/beneficiary.
    if !disable_coinbase_tip {
        mainnet::reward_beneficiary::<SPEC>(self.data, gas, gas_refund);
    }

    if self.data.env.cfg.optimism && !is_deposit {
        // If the transaction is not a deposit transaction, fees are paid out
        // to both the Base Fee Vault as well as the L1 Fee Vault.
        let Some(l1_block_info) = l1_block_info else {
            panic!("[OPTIMISM] Failed to load L1 block information.");
        };

        let Some(enveloped_tx) = &self.data.env.tx.optimism.enveloped_tx else {
            panic!("[OPTIMISM] Failed to load enveloped transaction.");
        };

        let l1_cost = l1_block_info.calculate_tx_l1_cost::<SPEC>(enveloped_tx, is_deposit);

        // Send the L1 cost of the transaction to the L1 Fee Vault.
        let Ok((l1_fee_vault_account, _)) = self
            .data
            .journaled_state
            .load_account(optimism::L1_FEE_RECIPIENT, self.data.db)
        else {
            panic!("[OPTIMISM] Failed to load L1 Fee Vault account");
        };
        l1_fee_vault_account.mark_touch();
        l1_fee_vault_account.info.balance += l1_cost;

        // Send the base fee of the transaction to the Base Fee Vault.
        let Ok((base_fee_vault_account, _)) = self
            .data
            .journaled_state
            .load_account(optimism::BASE_FEE_RECIPIENT, self.data.db)
        else {
            panic!("[OPTIMISM] Failed to load Base Fee Vault account");
        };
        base_fee_vault_account.mark_touch();
        base_fee_vault_account.info.balance +=
            l1_block_info.l1_base_fee.mul(U256::from(gas.spend()));
    }
}

#[cfg(feature = "optimism")]
#[cfg(test)]
mod tests {
    use crate::primitives::{BedrockSpec, RegolithSpec};

    use super::*;
    use crate::primitives::B256;

    #[test]
    fn test_revert_gas() {
        let mut env = Env::default();
        env.tx.gas_limit = 100;
        env.cfg.optimism = true;
        env.tx.optimism.source_hash = None;

        let gas = handle_call_return::<BedrockSpec>(&env, InstructionResult::Revert, Gas::new(90));
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spend(), 10);
        assert_eq!(gas.refunded(), 0);
    }

    #[test]
    fn test_revert_gas_non_optimism() {
        let mut env = Env::default();
        env.tx.gas_limit = 100;
        env.cfg.optimism = false;
        env.tx.optimism.source_hash = None;

        let gas = handle_call_return::<BedrockSpec>(&env, InstructionResult::Revert, Gas::new(90));
        // else branch takes all gas.
        assert_eq!(gas.remaining(), 0);
        assert_eq!(gas.spend(), 100);
        assert_eq!(gas.refunded(), 0);
    }

    #[test]
    fn test_consume_gas() {
        let mut env = Env::default();
        env.tx.gas_limit = 100;
        env.cfg.optimism = true;
        env.tx.optimism.source_hash = Some(B256::zero());

        let gas = handle_call_return::<RegolithSpec>(&env, InstructionResult::Stop, Gas::new(90));
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spend(), 10);
        assert_eq!(gas.refunded(), 0);
    }

    #[test]
    fn test_consume_gas_with_refund() {
        let mut env = Env::default();
        env.tx.gas_limit = 100;
        env.cfg.optimism = true;
        env.tx.optimism.source_hash = Some(B256::zero());

        let mut ret_gas = Gas::new(90);
        ret_gas.record_refund(20);

        let gas =
            handle_call_return::<RegolithSpec>(&env, InstructionResult::Stop, ret_gas.clone());
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spend(), 10);
        assert_eq!(gas.refunded(), 20);

        let gas = handle_call_return::<RegolithSpec>(&env, InstructionResult::Revert, ret_gas);
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spend(), 10);
        assert_eq!(gas.refunded(), 0);
    }

    #[test]
    fn test_consume_gas_sys_deposit_tx() {
        let mut env = Env::default();
        env.tx.gas_limit = 100;
        env.cfg.optimism = true;
        env.tx.optimism.source_hash = Some(B256::zero());

        let gas = handle_call_return::<BedrockSpec>(&env, InstructionResult::Stop, Gas::new(90));
        assert_eq!(gas.remaining(), 0);
        assert_eq!(gas.spend(), 100);
        assert_eq!(gas.refunded(), 0);
    }
}
