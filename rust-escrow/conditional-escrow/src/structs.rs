use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::near_bindgen;
use near_sdk::{AccountId, Balance};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ConditionalEscrow {
    pub deposits: UnorderedMap<AccountId, Balance>,
    pub expires_at: u64,
    pub total_funds: Balance,
    pub funding_amount_limit: u128,
    pub unpaid_funding_amount: u128,
    pub dao_factory_account_id: AccountId,
    pub ft_factory_account_id: AccountId,
    pub metadata_url: String,
    pub dao_name: String,
    pub is_dao_created: bool,
}
