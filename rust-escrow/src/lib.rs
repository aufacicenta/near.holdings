use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde_json::json;
use near_sdk::{assert_self, env, near_bindgen, AccountId, Gas, Promise};

const CONDITIONAL_ESCROW_CODE: &[u8] = include_bytes!("./conditional_escrow.wasm");

/// Gas spent on the call & account creation.
const CREATE_CALL_GAS: Gas = Gas(75_000_000_000_000);

/// Gas allocated on the callback.
const ON_CREATE_CALL_GAS: Gas = Gas(15_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct EscrowFactory {
    conditional_escrow_contracts: UnorderedSet<AccountId>,
}

impl Default for EscrowFactory {
    fn default() -> Self {
        env::panic_str("EscrowFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl EscrowFactory {
    #[init]
    pub fn new() -> Self {
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }
        
        Self {
            conditional_escrow_contracts: UnorderedSet::new(b"d".to_vec()),
        }
    }

    pub fn get_conditional_escrow_contracts_list(&self) -> Vec<AccountId> {
        self.conditional_escrow_contracts.to_vec()
    }

    /// Get number of created Conditional Escrow Contracts.
    pub fn get_conditional_escrow_contracts_count(&self) -> u64 {
        self.conditional_escrow_contracts.len()
    }

    /// Get Conditional Escrow Contracts in paginated view.
    pub fn get_conditional_escrow_contracts(&self, from_index: u64, limit: u64) -> Vec<AccountId> {
        let elements = self.conditional_escrow_contracts.as_vector();

        (from_index..std::cmp::min(from_index + limit, elements.len()))
            .filter_map(|index| elements.get(index))
            .collect()
    }

    #[payable]
    pub fn create_conditional_escrow(&mut self, name: AccountId, args: Base64VecU8) -> Promise {
        let account_id: AccountId = format!("{}.{}", name, env::current_account_id())
            .parse()
            .unwrap();

        let promise = Promise::new(account_id.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .deploy_contract(CONDITIONAL_ESCROW_CODE.to_vec())
            .transfer(env::attached_deposit())
            .function_call(
                "new".to_string(),
                args.into(),
                0,
                CREATE_CALL_GAS,
            );

        let callback = Promise::new(env::current_account_id())
            .function_call(
                "on_create_conditional_escrow".to_string(),
                json!({"account_id": account_id, "attached_deposit": U128(env::attached_deposit()), "predecessor_account_id": env::predecessor_account_id()})
                    .to_string()
                    .into_bytes(),
                0,
                ON_CREATE_CALL_GAS,
            );

        promise.then(callback)
    }

    pub fn on_create_conditional_escrow(
        &mut self,
        account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) {
        assert_self();

        if near_sdk::is_promise_success() {
            self.conditional_escrow_contracts.insert(&account_id);
        } else {
            let promise = Promise::new(predecessor_account_id).transfer(attached_deposit.0);

            let callback = Promise::new(env::current_account_id())
                .function_call(
                    "on_transfer_callback".to_string(),
                    json!({})
                        .to_string()
                        .into_bytes(),
                    0,
                    Gas(5_000_000_000_000),
                );

            promise.then(callback);
        }
    }

    pub fn on_transfer_callback(&self) {
        if env::promise_results_count() != 1 {
            env::panic_str("ERR_CALLBACK_METHOD");
        }

        env::panic_str("ERR_CREATE_CONDITIONAL_ESCROW_UNSUCCESSFUL");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use near_sdk::test_utils::test_env::alice;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::PublicKey;
    use near_sdk::{testing_env, PromiseResult};
    use serde_json::json;

    fn setup_contract() -> (VMContextBuilder, EscrowFactory) {
        let mut context = VMContextBuilder::new();
        let pk: PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
            .parse()
            .unwrap();
        testing_env!(context
            .signer_account_pk(pk)
            .current_account_id(alice())
            .build());
        let factory = EscrowFactory::new();
        (context, factory)
    }

    #[test]
    fn test_create_conditional_escrow() {
        let (mut context, mut factory) = setup_contract();

        let now = Utc::now().timestamp_nanos();
        let args = json!({ "expires_at": now, "funding_amount_limit": 1_000_000_000, "dao_factory_account_id": "daofactory.testnet", "ft_factory_account_id": "ftfactory.testnet", "metadata_url": "metadata_url.json" })
            .to_string()
            .into_bytes().to_vec().into();

        factory.create_conditional_escrow("conditional-escrow".parse().unwrap(), args);

        testing_env!(
            context.predecessor_account_id(alice()).build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        factory.on_create_conditional_escrow(
            format!("conditional-escrow.{}", alice()).parse().unwrap(),
            U128(0),
            alice(),
        );

        assert_eq!(
            factory.get_conditional_escrow_contracts_list(),
            vec![format!("conditional-escrow.{}", alice()).parse().unwrap()]
        );

        assert_eq!(
            factory.get_conditional_escrow_contracts(0, 100),
            vec![format!("conditional-escrow.{}", alice()).parse().unwrap()]
        );

        assert_eq!(factory.get_conditional_escrow_contracts_count(), 1);
    }

    #[test]
    #[should_panic(expected = "ERR_CREATE_CONDITIONAL_ESCROW_UNSUCCESSFUL")]
    fn test_create_conditional_escrow_fails() {
        let (mut context, mut factory) = setup_contract();

        let now = Utc::now().timestamp_nanos();
        let args = json!({ "expires_at": now, "funding_amount_limit": 1_000_000_000, "dao_factory_account_id": "daofactory.testnet", "ft_factory_account_id": "ftfactory.testnet", "metadata_url": "metadata_url.json" })
            .to_string()
            .into_bytes().to_vec().into();

        factory.create_conditional_escrow("conditional-escrow".parse().unwrap(), args);

        testing_env!(
            context.predecessor_account_id(alice()).build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Failed],
        );

        factory.on_create_conditional_escrow(
            format!("conditional-escrow.{}", alice()).parse().unwrap(),
            U128(0),
            alice(),
        );

        testing_env!(
            context.predecessor_account_id(alice()).build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        factory.on_transfer_callback();
    }
}
