use near_sdk::Balance;
use near_sdk::Gas;

/// Amount of gas
pub const GAS_FOR_CREATE_DAO: Gas = Gas(150_000_000_000_000);
pub const GAS_FOR_CREATE_FT: Gas = Gas(50_000_000_000_000);
pub const GAS_FOR_PROPOSAL: Gas = Gas(25_000_000_000_000);
pub const GAS_FOR_CALLBACK: Gas = Gas(2_000_000_000_000);

// Attached deposits
pub const FT_ATTACHED_DEPOSIT: Balance = 5_000_000_000_000_000_000_000_000; // 5 Near
