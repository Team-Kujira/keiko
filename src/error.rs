use cosmwasm_std::{ConversionOverflowError, OverflowError, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
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
    SaleAlreadyExists(String),

    #[error("Either the symbol or the denom is required, but not both")]
    DynomOrSymbolRequired {},

    #[error("Invalid Funds")]
    InvalidFunds {},

    #[error("Invalid Bid Denom")]
    InvalidBidDenom {},

    #[error("Minimum Raise Amount must be greater than {0}")]
    InvalidRaiseAmount(String),

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

    #[error("Category {0} requires a recipient")]
    RecipientRequired(String),

    #[error("Unknown Reply Id {0}")]
    UnknownReplyId(String),
}
