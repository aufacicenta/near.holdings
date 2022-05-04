use near_sdk::{env, near_bindgen, AccountId, Balance};

use crate::structs::*;

#[near_bindgen]
impl ConditionalEscrow {
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

    pub fn has_contract_expired(&self) -> bool {
        self.expires_at < env::block_timestamp().try_into().unwrap()
    }

    pub fn is_funding_reached(&self) -> bool {
        self.get_total_funds() >= self.get_funding_amount_limit()
    }

    pub fn get_deposit_accounts(&self) -> Vec<String> {
        let mut accounts = vec![];

        for i in self.deposits.to_vec() {
            accounts.push(i.0.to_string());
        }

        accounts
    }
}
