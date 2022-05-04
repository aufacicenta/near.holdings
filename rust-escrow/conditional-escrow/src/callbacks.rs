use near_sdk::{env, near_bindgen, PromiseResult};

use crate::structs::*;

#[near_bindgen]
impl ConditionalEscrow {
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
}
