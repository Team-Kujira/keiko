use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Coin, Order, StdResult, Storage, Uint128};

use crate::{
    msg::{Bow, Fin, LaunchStatus, Pilot, Token, Tokenomics},
    state::launch,
    ContractError,
};

#[cw_serde]
pub struct Launch {
    pub idx: Uint128,
    pub owner: Addr,
    pub deposit: Coin,
    pub status: LaunchStatus,
    pub token: Option<Token>,
    pub tokenomics: Option<Tokenomics>,
    pub pilot: Option<Pilot>,
    pub fin: Option<Fin>,
    pub bow: Option<Bow>,
}

impl Launch {
    fn next_idx(storage: &dyn Storage) -> Uint128 {
        match launch().keys(storage, None, None, Order::Descending).next() {
            Some(Ok(x)) => Uint128::from(x + 1),
            _ => Uint128::default(),
        }
    }

    pub fn new(storage: &dyn Storage, owner: Addr, deposit: Coin) -> Self {
        Self {
            idx: Self::next_idx(storage),
            owner,
            deposit,
            status: LaunchStatus::Created,
            token: None,
            tokenomics: None,
            pilot: None,
            fin: None,
            bow: None,
        }
    }

    pub fn load(storage: &dyn Storage, idx: Uint128) -> StdResult<Self> {
        launch().load(storage, idx.u128())
    }

    pub fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        launch().save(storage, self.idx.u128(), self)
    }

    pub fn is_owner(&self, addr: &Addr) -> Result<bool, ContractError> {
        ensure!(self.owner == addr, ContractError::Unauthorized {});
        Ok(true)
    }
}
