use std::fmt;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Decimal256, Uint128};
use fuzion_flows::FlowCreate;
use kujira::{CallbackMsg, Denom};
use kujira_pilot::{CreateOrca, CreateSale};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub token: TokenConfig,
    pub tokenomics: TokenomicsConfig,
    pub pilot: PilotConfig,
    pub flows: FlowsConfig,
    pub fin: FinConfig,
    pub bow: BowConfig,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// updates the config on the contract
    UpdateConfig {
        owner: Option<Addr>,
        token: Option<TokenConfig>,
        tokenomics: Option<TokenomicsConfig>,
        pilot: Option<PilotConfig>,
        flows: Option<FlowsConfig>,
        fin: Box<Option<FinConfig>>,
        bow: Option<BowConfig>,
    },
    /// creates a sale and requires the deposit to be paid
    Create {
        terms_conditions_accepted: bool,
    },
    /// creates or stores the token information for the launch
    Token {
        idx: Uint128,
        create: bool,
        symbol: Option<String>,
        denom: Option<Denom>,
        decimals: Option<u8>,
        denom_admin: Option<Addr>,
        png_url: Option<String>,
        svg_url: Option<String>,
    },
    /// sets up the tokenomics for the launch
    Tokenomics {
        idx: Uint128,
        categories: Vec<TokenomicsCategories>,
    },
    /// schedules the pilot sale with the required sale and orca information
    PilotSchedule {
        idx: Uint128,
        sale: CreateSale,
        orca: CreateOrca,
    },
    /// starts the pilot sale
    PilotStart {
        idx: Uint128,
    },
    PilotExecute {
        idx: Uint128,
    },
    PilotRetract {
        idx: Uint128,
    },
    PostLaunch {
        idx: Uint128,
    },
    FinLaunch {
        addr: Addr,
    },
    FinSetOwner {
        addr: Addr,
        owner: Addr,
    },
    Update {
        launch: crate::launch::Launch,
    },
    UpdateDescription {
        idx: Uint128,
        description: String,
    },
    Callback(CallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(crate::launch::Launch)]
    Launch { idx: Uint128 },
    #[returns(crate::launch::Launch)]
    LaunchByPilotIdx { idx: Uint128 },
    #[returns(Vec<crate::launch::Launch>)]
    Launches {
        start_after: Option<Uint128>,
        limit: Option<u8>,
    },
    #[returns(Vec<crate::launch::Launch>)]
    LaunchesByOwner {
        owner: Addr,
        start_after: Option<Uint128>,
        limit: Option<u8>,
    },
    #[returns(Vec<crate::launch::Launch>)]
    LaunchesByStatus {
        status: LaunchStatus,
        start_after: Option<Uint128>,
        limit: Option<u8>,
    },
}

#[cw_serde]
pub struct Token {
    pub denom: Denom,
    pub symbol: String,
    pub decimals: u8,
    pub is_managed: bool,
    pub denom_admin: Option<Addr>,
    pub png_url: Option<String>,
    pub svg_url: Option<String>,
}

#[cw_serde]
pub struct Tokenomics {
    pub categories: Vec<TokenomicsCategories>,
}

#[cw_serde]
pub struct TokenomicsCategories {
    pub label: String,
    pub category_type: CategoryTypes,
    pub recipients: Vec<TokenomicsRecipient>,
}

#[cw_serde]
pub struct TokenomicsRecipient {
    pub amount: Uint128,
    pub address: Option<Addr>,
    pub flows: Option<Vec<FlowCreate>>,
}

#[cw_serde]
pub enum CategoryTypes {
    Sale,
    Liquidity,
    Standard,
}

#[cw_serde]
pub struct Pilot {
    pub idx: Option<Uint128>,
    pub beneficiary: Addr,
    pub sale: CreateSale,
    pub orca: CreateOrca,
}

#[cw_serde]
pub struct Fin {
    pub contract_address: Option<Addr>,
}

#[cw_serde]
pub struct Bow {
    pub contract_address: Option<Addr>,
}

#[cw_serde]
pub enum CallbackType {
    BowCallback { idx: Uint128 },
}

#[cw_serde]
pub struct TokenConfig {
    pub denom_fee: Coin,
    pub default_admin: Addr,
    pub utilities_contract: Addr,
}

#[cw_serde]
pub struct TokenomicsConfig {
    pub minimum_liquidity_one_side: Decimal,
    pub default_lp_vest_cliff: u64,
    pub default_lp_vest_duration: u64,
}

#[cw_serde]
pub enum LaunchStatus {
    Created = 1,
    Planned = 2,
    InProgress = 3,
    Completed = 4,
}

impl fmt::Display for LaunchStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cw_serde]
pub struct BidDenoms {
    pub denom: Denom,
    pub symbol: String,
    pub decimals: u8,
}

#[cw_serde]
pub struct PilotConfig {
    pub pilot_contract: Addr,
    pub allowed_bid_denoms: Vec<BidDenoms>,
    pub min_raise_amount: Uint128,
}

#[cw_serde]
pub struct FlowsConfig {
    pub flows_contract: Addr,
}

#[cw_serde]
pub struct FinConfig {
    pub code_id: u64,
    pub owner: Addr,
    pub admin: Addr,
    pub fee_maker: Decimal256,
    pub fee_taker: Decimal256,
    pub fee_address: Addr,
}

#[cw_serde]
pub struct BowConfig {
    pub code_id: u64,
    pub owner: Addr,
    pub admin: Addr,
    pub intervals: Vec<Decimal>,
    pub fee: Decimal,
    pub amp: Decimal,
}

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub token: TokenConfig,
    pub tokenomics: TokenomicsConfig,
    pub pilot: PilotConfig,
    pub flows: FlowsConfig,
    pub fin: FinConfig,
    pub bow: BowConfig,
}

#[cw_serde]
pub struct ReplyInfo {
    pub reply_type: ReplyTypes,
    pub idx: Uint128,
}

#[cw_serde]
pub enum ReplyTypes {
    Create = 1,
    Execute = 2,
    PostExecute = 3,
    Retract = 4,
}
