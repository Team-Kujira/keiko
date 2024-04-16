use cosmwasm_std::{ConversionOverflowError, OverflowError, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("unauthorized")]
    Unauthorized {},

    #[error("Terms and Conditions must be accepted before creating a sale")]
    TermsConditionsAccepted {},

    #[error("Token information already exists")]
    TokenAlreadyExists {},

    #[error("Token information must be created before tokenomics")]
    TokenDoesNotExist {},

    #[error("Decimals will be defaulted to 6 for created tokens")]
    DecimalsDefaulted {},

    #[error("Denom will be determined automatically. Do not provide a denom")]
    DenomNotAllowed {},

    #[error("invalid denom {0}")]
    InvalidDenom(String),

    #[error("Sale already exists for denom {0}")]
    SaleAlreadyExistsDenom(String),

    #[error("Sale already exists for owner {0}")]
    SaleAlreadyExistsOwner(String),

    #[error("Either the symbol or the denom is required, but not both")]
    DynomOrSymbolRequired {},

    #[error("Launch in Status that does not allow this action")]
    InvalidStatus {},

    #[error("Invalid Funds")]
    InvalidFunds {},

    #[error("Invalid Amount Deposited, expected {0} got {1}")]
    InvalidDeposit(String, String),

    #[error("Invalid Input combination for this Message. Please ensure {0}")]
    InvalidInput(String),

    #[error("Invalid Bid Denom")]
    InvalidBidDenom {},

    #[error("Minimum Raise Amount {0}, must be greater than {1}")]
    InvalidRaiseAmount(String, String),

    #[error("Tokenomics requires one sale category")]
    OneSaleCategory {},

    #[error("Tokenomics requires one sale category recipient")]
    OneSaleCategoryRecipient {},

    #[error("Tokenomics requires one liquidity category")]
    OneLiquidityCategory {},

    #[error("Tokenomics requires one liquidity category recipient")]
    OneLiquidityCategoryRecipient {},

    #[error("Liquidity Amount can not be greater than Sale Amount")]
    LiquidityAmountSaleAmount {},

    #[error("No LP tokens received")]
    LPTokensNotReceived {},

    #[error("Liquidity Amount needs to be {0} of the Sale Amount")]
    LiquidityAmountBelowRequired(String),

    #[error("Category {0} must not have a recipient address or flow")]
    RecipientNotRequired(String),

    #[error("Category {0} requires a recipient")]
    RecipientRequired(String),

    #[error("Category {0} requires a recipient address or flow")]
    RecipientAddressOrFlowRequired(String),

    #[error("Category {0} requires a recipient address or flow, not both")]
    RecipientAddressAndFlow(String),

    #[error("Category {0} flows amount {1} do not equal the recipient amount {2}")]
    FlowsInvalidAmount(String, String, String),

    #[error("Unknown Reply Id {0}")]
    UnknownReplyId(String),
}
