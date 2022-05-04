#[cfg(test)]
mod tests {
    use crate::structs::ConditionalEscrow;
    use chrono::Utc;
    use near_sdk::json_types::U128;
    use near_sdk::test_utils::test_env::{alice, bob, carol};
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, PromiseResult};

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
