use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::{env, log, near_bindgen, Gas};
use near_sdk::{AccountId, Balance, Promise, PromiseResult};

/// Amount of gas
pub const GAS_FOR_CREATE_DAO: Gas = Gas(150_000_000_000_000);
pub const GAS_FOR_CREATE_FT: Gas = Gas(50_000_000_000_000);
pub const GAS_FOR_PROPOSAL: Gas = Gas(25_000_000_000_000);
pub const GAS_FOR_CALLBACK: Gas = Gas(2_000_000_000_000);

// Attached deposits
pub const FT_ATTACHED_DEPOSIT: Balance = 5_000_000_000_000_000_000_000_000; // 5 Near

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ConditionalEscrow {
    deposits: UnorderedMap<AccountId, Balance>,
    expires_at: u64,
    total_funds: Balance,
    funding_amount_limit: u128,
    unpaid_funding_amount: u128,
    dao_factory_account_id: AccountId,
    ft_factory_account_id: AccountId,
    metadata_url: String,
    dao_name: String,
    is_dao_created: bool,
}

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

    pub fn deposits_of(&self, payee: &AccountId) -> Balance {
        match self.deposits.get(payee) {
            Some(deposit) => deposit,
            None => 0,
        }
    }

    pub fn get_shares_of(&self, payee: &AccountId) -> Balance {
        match self.deposits.get(payee) {
            Some(deposit) => deposit * 1000 / self.funding_amount_limit,
            None => 0,
        }
    }

    pub fn get_deposits(&self) -> Vec<(AccountId, Balance)> {
        self.deposits.to_vec()
    }

    pub fn get_total_funds(&self) -> Balance {
        self.total_funds
    }

    pub fn get_metadata_url(&self) -> String {
        self.metadata_url.clone()
    }

    pub fn get_expiration_date(&self) -> u64 {
        self.expires_at
    }

    pub fn get_funding_amount_limit(&self) -> u128 {
        self.funding_amount_limit
    }

    pub fn get_unpaid_funding_amount(&self) -> u128 {
        self.unpaid_funding_amount
    }

    pub fn get_dao_factory_account_id(&self) -> AccountId {
        self.dao_factory_account_id.clone()
    }

    pub fn get_ft_factory_account_id(&self) -> AccountId {
        self.ft_factory_account_id.clone()
    }

    pub fn get_dao_name(&self) -> String {
        self.dao_name.clone()
    }

    pub fn is_deposit_allowed(&self) -> bool {
        !self.has_contract_expired() && !self.is_funding_reached()
    }

    pub fn is_withdrawal_allowed(&self) -> bool {
        self.has_contract_expired() && !self.is_funding_reached()
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
            "{} deposited {} NEAR tokens. New balance {} ?????Total funds: {} ?????Unpaid funds: {}",
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
            "{} withdrawn {} NEAR tokens. New balance {} ?????Total funds: {} ?????Unpaid funds: {}",
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

    #[private]
    pub fn on_delegate_callback(&mut self, dao_name: String) -> bool {
        if env::promise_results_count() != 2 {
            env::panic_str("ERR_CALLBACK_METHOD");
        }

        let on_create_dao_successful;
        let on_create_ft_successful;

        // Create DAO Contract
        match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                let res: bool = near_sdk::serde_json::from_slice(&result).unwrap();

                if res {
                    self.total_funds = 0;
                    self.dao_name = dao_name;
                    self.is_dao_created = true;
                    on_create_dao_successful = true;
                } else {
                    on_create_dao_successful = false;
                }
            }
            _ => env::panic_str("ERR_CREATE_DAO_UNSUCCESSFUL"),
        }

        // Create FT Contract
        match env::promise_result(1) {
            PromiseResult::Successful(result) => {
                let res: bool = near_sdk::serde_json::from_slice(&result).unwrap();

                if res {
                    on_create_ft_successful = true;
                } else {
                    on_create_ft_successful = false;
                }
            }
            _ => env::panic_str("ERR_CREATE_FT_UNSUCCESSFUL"),
        }

        on_create_dao_successful && on_create_ft_successful
    }

    fn has_contract_expired(&self) -> bool {
        self.expires_at < env::block_timestamp().try_into().unwrap()
    }

    fn is_funding_reached(&self) -> bool {
        self.get_total_funds() >= self.get_funding_amount_limit()
    }

    fn get_deposit_accounts(&self) -> Vec<String> {
        let mut accounts = vec![];

        for i in self.deposits.to_vec() {
            accounts.push(i.0.to_string());
        }

        accounts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use near_sdk::test_utils::test_env::{alice, bob, carol};
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, PromiseResult};

    const ATTACHED_DEPOSIT: Balance = 1_000_000_000_000_000_000_000_000; // 1 Near
    const MIN_FUNDING_AMOUNT: Balance = 15_000_000_000_000_000_000_000_000; // 15 Near

    fn setup_context() -> VMContextBuilder {
        let mut context = VMContextBuilder::new();
        let now = Utc::now().timestamp_subsec_nanos();
        testing_env!(context
            .predecessor_account_id(alice())
            .block_timestamp(now.try_into().unwrap())
            .build());

        context
    }

    fn setup_contract(expires_at: u64, funding_amount_limit: u128) -> ConditionalEscrow {
        let contract = ConditionalEscrow::new(
            expires_at,
            U128(funding_amount_limit),
            accounts(3),
            accounts(4),
            "metadata_url.json".to_string(),
        );

        contract
    }

    fn add_expires_at_nanos(offset: u32) -> u64 {
        let now = Utc::now().timestamp_subsec_nanos();
        (now + offset).into()
    }

    fn substract_expires_at_nanos(offset: u32) -> u64 {
        let now = Utc::now().timestamp_subsec_nanos();
        (now - offset).into()
    }

    #[test]
    #[should_panic(expected = "ERR_INSUFFICIENT_FUNDS_LIMIT")]
    fn test_new_fail() {
        let mut context = setup_context();

        testing_env!(context.signer_account_id(bob()).attached_deposit(0).build());

        let expires_at = add_expires_at_nanos(100);

        // Should fail because insufficient funds limit
        ConditionalEscrow::new(
            expires_at,
            U128(1_000_000_000_000_000_000_000_000), // 1 NEAR
            accounts(3),
            accounts(4),
            "metadata_url.json".to_string(),
        );
    }

    #[test]
    fn test_get_deposits_of() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(
            0,
            contract.deposits_of(&alice()),
            "Account deposits should be 0"
        );
    }

    #[test]
    fn test_get_shares_of() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        assert_eq!(
            0,
            contract.get_shares_of(&alice()),
            "Account deposits should be 0"
        );

        assert_eq!(
            ATTACHED_DEPOSIT * 1000 / contract.funding_amount_limit,
            contract.get_shares_of(&bob()),
            "Proportion deposit of Bob should be 8"
        );
    }

    #[test]
    fn test_get_deposits() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        assert_eq!(
            contract.get_deposits(),
            vec![(bob(), ATTACHED_DEPOSIT)],
            "Gets all deposits as vec"
        );
    }

    #[test]
    fn test_get_deposit_accounts() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            vec!["bob.near", "carol.near"],
            contract.get_deposit_accounts(),
        );
    }

    #[test]
    fn test_get_dao_factory_account_id() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(
            accounts(3),
            contract.get_dao_factory_account_id(),
            "Recipient account id should be 'danny.near'"
        );
    }

    #[test]
    fn test_get_ft_factory_account_id() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(
            accounts(4),
            contract.get_ft_factory_account_id(),
            "Recipient account id should be 'eugene.near'"
        );
    }

    #[test]
    fn test_get_dao_name() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        contract.delegate_funds("dao1".to_string());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![
                PromiseResult::Successful("true".to_string().into_bytes()),
                PromiseResult::Successful("true".to_string().into_bytes())
            ],
        );

        assert_eq!(
            contract.on_delegate_callback("dao1".to_string()),
            true,
            "delegate_funds should run successfully"
        );

        assert_eq!("dao1", contract.get_dao_name(), "Should equal DAO Name");
    }

    #[test]
    fn test_get_metadata_url() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(
            "metadata_url.json",
            contract.get_metadata_url(),
            "Contract was not initilialized with metadata_url param"
        );
    }

    #[test]
    fn test_get_0_total_funds() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");
    }

    #[test]
    fn test_get_correct_unpaid_funding_amount() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        assert_eq!(
            MIN_FUNDING_AMOUNT - ATTACHED_DEPOSIT,
            contract.get_unpaid_funding_amount(),
            "Unpaid funding amount is wrong"
        );
    }

    #[test]
    fn test_get_total_funds_after_deposits() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            ATTACHED_DEPOSIT * 2,
            contract.get_total_funds(),
            "Total funds should be ATTACHED_DEPOSITx2"
        );
    }

    #[test]
    fn test_is_withdrawal_allowed() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT * 2);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT - 1_000)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT - 1_000)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(bob())
            .block_timestamp((expires_at + 100).try_into().unwrap())
            .build());

        contract.withdraw();

        testing_env!(context.signer_account_id(carol()).build());

        contract.withdraw();

        assert_eq!(
            true,
            contract.is_withdrawal_allowed(),
            "Withdrawal should be allowed"
        );

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");
    }

    #[test]
    #[should_panic(expected = "ERR_DEPOSIT_SHOULD_NOT_BE_0")]
    fn test_deposits() {
        let mut context = setup_context();

        testing_env!(context.signer_account_id(bob()).attached_deposit(0).build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();
    }

    #[test]
    #[should_panic(expected = "ERR_WITHDRAWAL_NOT_ALLOWED")]
    fn test_is_withdrawal_not_allowed() {
        setup_context();
        let expires_at = add_expires_at_nanos(1_000_000);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.withdraw();

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_DEPOSIT_NOT_ALLOWED")]
    fn test_is_deposit_not_allowed_by_expiration_date() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = substract_expires_at_nanos(5_000_000);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_DEPOSIT_NOT_ALLOWED")]
    fn test_is_deposit_not_allowed_by_total_funds_reached() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT)
            .build());

        let expires_at = add_expires_at_nanos(1_000_000);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_OWNER_SHOULD_NOT_DEPOSIT")]
    fn test_owner_deposit() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(alice())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();
    }

    #[test]
    #[should_panic(expected = "ERR_DELEGATE_NOT_ALLOWED")]
    fn test_should_not_delegate_funds_if_active() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            true,
            contract.is_deposit_allowed(),
            "Deposit should be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_DELEGATE_NOT_ALLOWED")]
    fn test_should_not_delegate_funds_if_expired() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT - 1_000)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            true,
            contract.is_withdrawal_allowed(),
            "Withdrawal should be allowed"
        );

        contract.delegate_funds("dao1".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_DELEGATE_NOT_ALLOWED")]
    fn test_should_not_delegate_funds_if_already_delegated() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![
                PromiseResult::Successful("true".to_string().into_bytes()),
                PromiseResult::Successful("true".to_string().into_bytes())
            ],
        );

        assert_eq!(
            contract.on_delegate_callback("dao1".to_string()),
            true,
            "delegate_funds should run successfully"
        );

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");

        contract.delegate_funds("dao1".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_CREATE_DAO_UNSUCCESSFUL")]
    fn test_should_not_delegate_funds_if_create_dao_fails() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Failed, PromiseResult::Failed],
        );

        assert_eq!(
            contract.on_delegate_callback("dao1".to_string()),
            false,
            "delegate_funds should fail"
        );

        assert_eq!(
            MIN_FUNDING_AMOUNT,
            contract.get_total_funds(),
            "Total funds should be MIN_FUNDING_AMOUNT"
        );
    }

    #[test]
    fn test_delegate_funds() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![
                PromiseResult::Successful("true".to_string().into_bytes()),
                PromiseResult::Successful("true".to_string().into_bytes())
            ],
        );

        assert_eq!(
            contract.on_delegate_callback("dao1".to_string()),
            true,
            "delegate_funds should run successfully"
        );

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");

        assert_eq!(
            MIN_FUNDING_AMOUNT / 2,
            contract.deposits_of(&bob()),
            "Account deposits should be MIN_FUNDING_AMOUNT"
        );

        assert_eq!(
            MIN_FUNDING_AMOUNT / 2,
            contract.deposits_of(&carol()),
            "Account deposits should be MIN_FUNDING_AMOUNT"
        );
    }
}
