//! This contract provides an interface to Pilot sales and orchestrates the
//! creation of DeFi instruments as required by the launcher
pub mod contract;
pub mod error;
pub mod msg;
pub mod launch;
pub mod state;

pub use crate::error::ContractError;
pub use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
