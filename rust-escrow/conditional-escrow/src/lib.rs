use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, log, near_bindgen};
use near_sdk::{AccountId, Balance, Promise};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ConditionalEscrow {
    deposits: UnorderedMap<AccountId, Balance>,
    expires_at: u64,
    total_funds: Balance,
    min_funding_amount: u128,
    recipient_account_id: AccountId,
}

impl Default for ConditionalEscrow {
    fn default() -> Self {
        env::panic_str("ConditionalEscrow should be initialized before usage")
    }
}

#[near_bindgen]
impl ConditionalEscrow {
    #[init]
    pub fn new(expires_at: u64, min_funding_amount: u128, recipient_account_id: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            deposits: UnorderedMap::new(b"r".to_vec()),
            total_funds: 0,
            expires_at,
            min_funding_amount,
            recipient_account_id,
        }
    }

    pub fn deposits_of(&self, payee: &AccountId) -> Balance {
        match self.deposits.get(payee) {
            Some(deposit) => deposit,
            None => 0,
        }
    }

    pub fn get_deposits(&self) -> Vec<(AccountId, Balance)> {
        self.deposits.to_vec()
    }

    pub fn get_total_funds(&self) -> Balance {
        self.total_funds
    }

    pub fn get_expiration_date(&self) -> u64 {
        self.expires_at
    }

    pub fn get_min_funding_amount(&self) -> u128 {
        self.min_funding_amount
    }

    pub fn get_recipient_account_id(&self) -> AccountId {
        self.recipient_account_id.clone()
    }

    pub fn is_deposit_allowed(&self) -> bool {
        !self.has_contract_expired()
    }

    pub fn is_withdrawal_allowed(&self) -> bool {
        self.has_contract_expired() && !self.is_funding_minimum_reached()
    }

    #[payable]
    pub fn deposit(&mut self) {
        assert_ne!(
            env::current_account_id(),
            env::signer_account_id(),
            "ERR_OWNER_SHOULD_NOT_DEPOSIT"
        );

        assert!(self.is_deposit_allowed(), "ERR_DEPOSIT_NOT_ALLOWED");

        let amount = env::attached_deposit();
        let payee = env::signer_account_id();
        let current_balance = self.deposits_of(&payee);
        let new_balance = &(current_balance.wrapping_add(amount));

        self.deposits.insert(&payee, new_balance);
        self.total_funds = self.total_funds.wrapping_add(amount);

        log!(
            "{} deposited {} NEAR tokens. New balance {} — Total funds: {}",
            &payee,
            amount,
            new_balance,
            self.total_funds
        );
        // @TODO emit deposit event
    }

    #[payable]
    pub fn withdraw(&mut self) {
        assert!(self.is_withdrawal_allowed(), "ERR_WITHDRAWAL_NOT_ALLOWED");

        let payee = env::signer_account_id();
        let payment = self.deposits_of(&payee);

        Promise::new(payee.clone()).transfer(payment);
        self.deposits.insert(&payee, &0);
        self.total_funds = self.total_funds.wrapping_sub(payment);

        log!(
            "{} withdrawn {} NEAR tokens. New balance {} — Total funds: {}",
            &payee,
            payment,
            self.deposits_of(&payee),
            self.total_funds
        );
        // @TODO emit withdraw event
    }

    #[payable]
    pub fn delegate_funds(&mut self) {
        assert!(
            !(self.is_deposit_allowed() || self.is_withdrawal_allowed()),
            "ERR_DELEGATE_NOT_ALLOWED"
        );

        let payee = self.get_recipient_account_id();
        let total_funds = self.get_total_funds();

        Promise::new(payee.clone()).transfer(total_funds);
        self.total_funds = 0;

        log!(
            "Delegating {} NEAR tokens to {}. — Total funds held after call: {}",
            total_funds,
            payee,
            self.get_total_funds()
        );
        // @TODO emit delegate_funds event
    }

    fn has_contract_expired(&self) -> bool {
        self.expires_at < env::block_timestamp().try_into().unwrap()
    }

    fn is_funding_minimum_reached(&self) -> bool {
        self.get_total_funds() >= self.get_min_funding_amount()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use near_sdk::test_utils::test_env::{alice, bob, carol};
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    const ATTACHED_DEPOSIT: Balance = 8_540_000_000_000_000_000_000;
    const MIN_FUNDING_AMOUNT: u128 = 1_000_000_000_000_000_000_000_000;

    fn setup_context() -> VMContextBuilder {
        let mut context = VMContextBuilder::new();
        let now = Utc::now().timestamp_subsec_nanos();
        testing_env!(context
            .predecessor_account_id(alice())
            .block_timestamp(now.try_into().unwrap())
            .build());
        return context;
    }

    fn setup_contract(expires_at: u64, min_funding_amount: u128) -> ConditionalEscrow {
        let contract = ConditionalEscrow::new(expires_at, min_funding_amount, accounts(3));
        return contract;
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
    fn test_get_recipient_account_id() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(
            accounts(3),
            contract.get_recipient_account_id(),
            "Recipient account id should be 'danny.near'"
        );
    }

    #[test]
    fn test_get_0_total_funds() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");
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
    fn test_is_deposit_not_allowed() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT)
            .build());

        let expires_at = substract_expires_at_nanos(1_000_000);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

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

        contract.delegate_funds();
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

        contract.delegate_funds();
    }

    #[test]
    fn test_delegate_funds() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT)
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

        contract.delegate_funds();

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");

        assert_eq!(
            MIN_FUNDING_AMOUNT,
            contract.deposits_of(&bob()),
            "Account deposits should be MIN_FUNDING_AMOUNT"
        );

        assert_eq!(
            MIN_FUNDING_AMOUNT,
            contract.deposits_of(&carol()),
            "Account deposits should be MIN_FUNDING_AMOUNT"
        );
    }
}
