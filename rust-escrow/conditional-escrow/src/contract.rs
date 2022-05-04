use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{env, log, near_bindgen};
use near_sdk::{AccountId, Promise};

use crate::consts::*;
use crate::structs::*;

impl Default for ConditionalEscrow {
    fn default() -> Self {
        env::panic_str("ConditionalEscrow should be initialized before usage")
    }
}

#[near_bindgen]
impl ConditionalEscrow {
    #[init]
    pub fn new(
        expires_at: u64,
        funding_amount_limit: U128,
        dao_factory_account_id: AccountId,
        ft_factory_account_id: AccountId,
        metadata_url: String,
    ) -> Self {
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }

        if funding_amount_limit.0 < FT_ATTACHED_DEPOSIT {
            env::panic_str("ERR_INSUFFICIENT_FUNDS_LIMIT");
        }

        Self {
            deposits: UnorderedMap::new(b"r".to_vec()),
            total_funds: 0,
            funding_amount_limit: funding_amount_limit.0,
            unpaid_funding_amount: funding_amount_limit.0,
            expires_at,
            dao_factory_account_id,
            ft_factory_account_id,
            metadata_url,
            dao_name: "".to_string(),
            is_dao_created: false,
        }
    }

    #[payable]
    pub fn deposit(&mut self) {
        if env::current_account_id() == env::signer_account_id() {
            env::panic_str("ERR_OWNER_SHOULD_NOT_DEPOSIT");
        }

        if env::attached_deposit() == 0 {
            env::panic_str("ERR_DEPOSIT_SHOULD_NOT_BE_0");
        }

        if !self.is_deposit_allowed() {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        if env::attached_deposit() > self.get_unpaid_funding_amount() {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        let amount = env::attached_deposit();
        let payee = env::signer_account_id();
        let current_balance = self.deposits_of(&payee);
        let new_balance = &(current_balance.wrapping_add(amount));

        self.deposits.insert(&payee, new_balance);
        self.total_funds = self.total_funds.wrapping_add(amount);
        self.unpaid_funding_amount = self.unpaid_funding_amount.wrapping_sub(amount);

        log!(
            "{} deposited {} NEAR tokens. New balance {} — Total funds: {} — Unpaid funds: {}",
            &payee,
            amount,
            new_balance,
            self.total_funds,
            self.unpaid_funding_amount
        );
        // @TODO emit deposit event
    }

    #[payable]
    pub fn withdraw(&mut self) {
        if !self.is_withdrawal_allowed() {
            env::panic_str("ERR_WITHDRAWAL_NOT_ALLOWED");
        }

        let payee = env::signer_account_id();
        let payment = self.deposits_of(&payee);

        Promise::new(payee.clone()).transfer(payment);
        self.deposits.insert(&payee, &0);
        self.total_funds = self.total_funds.wrapping_sub(payment);
        self.unpaid_funding_amount = self.unpaid_funding_amount.wrapping_add(payment);

        log!(
            "{} withdrawn {} NEAR tokens. New balance {} — Total funds: {} — Unpaid funds: {}",
            &payee,
            payment,
            self.deposits_of(&payee),
            self.total_funds,
            self.unpaid_funding_amount
        );
        // @TODO emit withdraw event
    }

    #[payable]
    pub fn delegate_funds(&mut self, dao_name: String) -> Promise {
        if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
            env::panic_str("ERR_DELEGATE_NOT_ALLOWED");
        }

        if self.total_funds.checked_sub(FT_ATTACHED_DEPOSIT) == None {
            env::panic_str("ERR_TOTAL_FUNDS_OVERFLOW");
        }

        // @TODO charge a fee here (1.5% initially?) when a property is sold by our contract

        let dao_promise = Promise::new(self.dao_factory_account_id.clone()).function_call(
            "create_dao".to_string(),
            json!({"dao_name": dao_name.clone(), "deposits": self.get_deposit_accounts() })
                .to_string()
                .into_bytes(),
            self.total_funds - FT_ATTACHED_DEPOSIT,
            GAS_FOR_CREATE_DAO,
        );

        let ft_promise = Promise::new(self.ft_factory_account_id.clone()).function_call(
            "create_ft".to_string(),
            json!({"name": dao_name.clone()}).to_string().into_bytes(),
            FT_ATTACHED_DEPOSIT,
            GAS_FOR_CREATE_FT,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_delegate_callback".to_string(),
            json!({"dao_name": dao_name.clone()})
                .to_string()
                .into_bytes(),
            0,
            GAS_FOR_CALLBACK,
        );

        dao_promise.and(ft_promise).then(callback)

        // @TODO emit delegate_funds event
    }
}
