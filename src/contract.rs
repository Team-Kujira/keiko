use std::ops::{Add, AddAssign, Mul, Sub};
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, coins, ensure, from_json, instantiate2_address, to_json_binary, wasm_execute, BankMsg,
    Binary, CodeInfoResponse, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    Reply, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use cw_utils::{NativeBalance, PaymentError};
use fuzion_flows::{FlowCreate, FlowSchedule, FlowType};
use fuzion_utilities::{Asset, AssetList, DenomUnit, LogoURIs};
use kujira::{DenomMsg, KujiraMsg, KujiraQuery, Precision};
use kujira_pilot::Status;

use crate::launch::Launch;
use crate::msg::{
    Bow, CallbackType, CategoryTypes, Config, Fin, LaunchStatus, Pilot, ReplyInfo, ReplyTypes,
    Token, Tokenomics,
};
use crate::state::{launch, CONFIG, REPLY};
use crate::{ContractError, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

const CONTRACT_NAME: &str = "fuzion-kujira-keiko";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn migrate(deps: DepsMut<KujiraQuery>, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut<KujiraQuery>,
    _: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: msg.owner,
        token: msg.token,
        tokenomics: msg.tokenomics,
        pilot: msg.pilot,
        flows: msg.flows,
        fin: msg.fin,
        bow: msg.bow,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<KujiraQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            token,
            tokenomics,
            pilot,
            flows,
            fin,
            bow,
        } => {
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});
            if let Some(owner) = owner {
                config.owner = owner;
            }
            if let Some(token) = token {
                config.token = token;
            }
            if let Some(tokenomics) = tokenomics {
                config.tokenomics = tokenomics;
            }
            if let Some(pilot) = pilot {
                config.pilot = pilot;
            }
            if let Some(flows) = flows {
                config.flows = flows;
            }
            if let Some(fin) = *fin {
                config.fin = fin;
            }
            if let Some(bow) = bow {
                config.bow = bow;
            }
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::default())
        }
        ExecuteMsg::Create {
            terms_conditions_accepted,
        } => {
            // Sets up the launch and takes the deposit
            let pilot_config: kujira_pilot::ConfigResponse = deps.querier.query_wasm_smart(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::QueryMsg::Config {},
            )?;

            ensure!(
                info.funds.len() == 1 && info.funds[0] == pilot_config.deposit,
                ContractError::Payment(PaymentError::MissingDenom(pilot_config.deposit.denom))
            );

            ensure!(
                terms_conditions_accepted,
                ContractError::TermsConditionsAccepted {}
            );

            let existing_sale: Vec<Launch> = from_json(
                query(
                    deps.as_ref(),
                    env,
                    QueryMsg::LaunchesByOwner {
                        owner: info.sender.clone(),
                        start_after: None,
                        limit: None,
                    },
                )
                .unwrap(),
            )
            .unwrap();

            ensure!(
                existing_sale.is_empty(),
                ContractError::SaleAlreadyExistsOwner(info.sender.to_string())
            );

            let launch = Launch::new(
                deps.storage,
                info.sender.clone(),
                info.funds[0].clone(),
                terms_conditions_accepted,
            );
            launch.save(deps.storage)?;
            Ok(Response::default().add_attribute("action", "create"))
        }
        ExecuteMsg::Token {
            idx,
            create,
            denom,
            symbol,
            decimals,
            denom_admin,
            png_url,
            svg_url,
        } => {
            // Creates or stores the token information for the launch
            let mut launch = Launch::load(deps.storage, idx)?;
            launch.is_owner(&info.sender)?;

            ensure!(
                launch.status == LaunchStatus::Created,
                ContractError::InvalidStatus {}
            );

            if create {
                ensure!(
                    info.funds.len() == 1 && info.funds[0] == config.token.denom_fee,
                    ContractError::InvalidFunds {}
                );
                ensure!(launch.token.is_none(), ContractError::TokenAlreadyExists {});
                ensure!(
                    denom.is_none() && symbol.is_some() && png_url.is_some() && svg_url.is_some(),
                    ContractError::InvalidInput(
                        "denom is not populated and symbol, png_url and svg_url are populated when create is true"
                            .to_string()
                    )
                );
            } else if launch.token.is_none() {
                ensure!(
                    denom.is_some() && symbol.is_some() && decimals.is_some() && png_url.is_some() && svg_url.is_some(),
                    ContractError::InvalidInput(
                        "denom, symbol, decimals, png_url and svg_url are populated when create is false".to_string()
                    )
                );
            }

            if let Some(denom) = denom.clone() {
                let balance = deps.querier.query_supply(denom.to_string());
                ensure!(
                    balance.is_ok() && balance.unwrap().amount.u128() > 0,
                    ContractError::InvalidDenom(denom.to_string())
                );
            }

            if let Some(token) = launch.token.clone() {
                if token.is_managed {
                    ensure!(
                        denom.is_none() && symbol.is_none() && decimals.is_none(),
                        ContractError::Unauthorized {}
                    );
                }
            }

            let mut messages: Vec<KujiraMsg> = vec![];

            let launch_denom = if create {
                ensure!(denom.is_none(), ContractError::DenomNotAllowed {});
                ensure!(decimals.is_none(), ContractError::DecimalsDefaulted {});

                let contract_address = env.contract.address.to_string();
                let symbol_lower = symbol.clone().unwrap().to_lowercase();

                // mints a new token
                let kujira_denom =
                    kujira::Denom::from(format!("factory/{contract_address}/u{symbol_lower}"));
                messages.push(KujiraMsg::Denom(DenomMsg::Create {
                    subdenom: kujira::Denom::from(format!("u{symbol_lower}").to_string()),
                }));
                kujira_denom
            } else {
                denom.unwrap()
            };

            let token_denom_admin = if let Some(denom_admin) = denom_admin.clone() {
                Some(deps.api.addr_validate(denom_admin.clone().as_ref())?)
            } else {
                None
            };
            let launch_token = Token {
                denom: launch_denom.clone(),
                is_managed: create,
                symbol: symbol.clone().unwrap(),
                decimals: if create { 6 } else { decimals.unwrap() },
                denom_admin: token_denom_admin,
                png_url,
                svg_url,
            };

            launch.token = Some(launch_token);
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "token")
                .add_attribute("denom", launch_denom.to_string())
                .add_messages(messages))
        }
        ExecuteMsg::Tokenomics { idx, categories } => {
            // sets up the tokenomics for the launch
            // requires at least one sale and one liquidity category so that pilot and Bow can be set up
            let mut launch = Launch::load(deps.storage, idx)?;
            launch.is_owner(&info.sender)?;

            let pilot_config: kujira_pilot::ConfigResponse = deps.querier.query_wasm_smart(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::QueryMsg::Config {},
            )?;

            ensure!(
                launch.token.is_some() && launch.pilot.is_none(),
                ContractError::Unauthorized {}
            );

            // ensure only one sale category
            let sale_category = categories
                .iter()
                .filter(|f| f.category_type == CategoryTypes::Sale);

            ensure!(
                sale_category.clone().count() == 1,
                ContractError::OneSaleCategory {}
            );

            ensure!(
                sale_category.clone().next().unwrap().recipients.len() == 1,
                ContractError::OneSaleCategoryRecipient {}
            );

            // ensure only one liquidity category
            let liquidity_category = categories
                .iter()
                .filter(|f| f.category_type == CategoryTypes::Liquidity);

            ensure!(
                liquidity_category.clone().count() == 1,
                ContractError::OneLiquidityCategory {}
            );

            ensure!(
                liquidity_category.clone().next().unwrap().recipients.len() == 1,
                ContractError::OneSaleCategoryRecipient {}
            );

            let max_liquidity = Decimal::from_atomics(
                sale_category.clone().next().unwrap().recipients[0].amount,
                0,
            )
            .unwrap()
            .mul(Decimal::one() - pilot_config.sale_fee);

            ensure!(
                liquidity_category.clone().next().unwrap().recipients[0]
                    .amount
                    .le(&max_liquidity.to_uint_floor()),
                ContractError::LiquidityAmountSaleAmount(max_liquidity.to_string())
            );

            ensure!(
                liquidity_category.into_iter().next().unwrap().recipients[0]
                    .amount
                    .ge(&sale_category.into_iter().next().unwrap().recipients[0]
                        .amount
                        .mul(config.tokenomics.minimum_liquidity_one_side)),
                ContractError::LiquidityAmountBelowRequired(
                    config.tokenomics.minimum_liquidity_one_side.to_string()
                )
            );

            // ensure that each category has a recipient
            for category in categories.clone() {
                for recipient in category.recipients {
                    if category.category_type == CategoryTypes::Sale
                        || category.category_type == CategoryTypes::Liquidity
                    {
                        ensure!(
                            recipient.address.is_none() && recipient.flows.is_none(),
                            ContractError::RecipientNotRequired(category.label)
                        );
                    } else {
                        if recipient.address.is_none() && recipient.flows.is_none() {
                            return Err(ContractError::RecipientAddressOrFlowRequired(
                                category.label,
                            ));
                        }
                        if recipient.address.is_some() && recipient.flows.is_some() {
                            return Err(ContractError::RecipientAddressAndFlow(category.label));
                        }
                    }

                    if let Some(flows) = recipient.flows {
                        let mut flows_sum = 0u128;
                        for flow in flows {
                            for schedule in flow.schedules {
                                flows_sum = flows_sum.add(schedule.amount.u128());
                            }
                        }
                        ensure!(
                            flows_sum == recipient.amount.u128(),
                            ContractError::FlowsInvalidAmount(
                                category.label,
                                flows_sum.to_string(),
                                recipient.amount.to_string(),
                            )
                        );
                    }
                }
            }

            let tokenomics: Tokenomics = Tokenomics { categories };
            launch.tokenomics = Some(tokenomics);
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "tokenomics")
                .add_attribute("idx", idx))
        }
        ExecuteMsg::PilotSchedule { idx, sale, orca } => {
            // Schedule the pilot sale and set the pilot status to planned
            // Does not create the Pilot Sale but stores the information for the pilot contract
            let mut launch = Launch::load(deps.storage, idx)?;
            launch.is_owner(&info.sender)?;

            ensure!(
                launch.token.is_some()
                    && launch.tokenomics.is_some()
                    && (launch.pilot.is_none()
                        || (launch.pilot.is_some() && launch.status == LaunchStatus::Planned)),
                ContractError::Unauthorized {}
            );

            let bid_denom = config
                .pilot
                .allowed_bid_denoms
                .iter()
                .find(|d| d.denom == orca.bid_denom.clone());

            ensure!(bid_denom.is_some(), ContractError::InvalidBidDenom {});

            let categories = launch.clone().tokenomics.unwrap().categories;

            let sale_category = categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Sale)
                .unwrap();

            let launch_min_raise_amount = sale_category.recipients[0].amount.mul(sale.price).mul(
                Decimal::from_str(&orca.max_slot.to_string())
                    .unwrap()
                    .mul(orca.premium_rate_per_slot),
            );

            ensure!(
                launch_min_raise_amount > config.pilot.min_raise_amount,
                ContractError::InvalidRaiseAmount(
                    launch_min_raise_amount.to_string(),
                    config.pilot.min_raise_amount.to_string()
                )
            );

            let mut pilot = Pilot {
                idx: None,
                beneficiary: sale.beneficiary.clone(),
                sale,
                orca,
            };
            pilot.sale.beneficiary = env.contract.address;

            launch.pilot = Some(pilot);
            launch.status = LaunchStatus::Planned;
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "pilot_schedule")
                .add_attribute("idx", idx))
        }
        ExecuteMsg::PilotStart { idx } => {
            // Starts the pilot sale by creating the sale on the pilot contract and sets the status to in progress
            let mut launch = Launch::load(deps.storage, idx)?;
            launch.is_owner(&info.sender)?;

            let pilot_config: kujira_pilot::ConfigResponse = deps.querier.query_wasm_smart(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::QueryMsg::Config {},
            )?;

            ensure!(
                launch.token.clone().is_some()
                    && launch.tokenomics.clone().is_some()
                    && launch.pilot.clone().is_some()
                    && launch.status == LaunchStatus::Planned
                    && launch.pilot.clone().unwrap().idx.is_none()
                    && launch.pilot.clone().unwrap().sale.opens.seconds()
                        <= env.block.time.seconds(),
                ContractError::Unauthorized {}
            );

            let categories = launch.clone().tokenomics.unwrap().categories;

            let sale_category = categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Sale)
                .unwrap();

            let mut messages = vec![];
            let denom = launch.clone().token.unwrap().denom;

            let sale_funds = if launch.clone().token.unwrap().is_managed {
                messages.push(SubMsg::new(CosmosMsg::Custom(KujiraMsg::Denom(
                    DenomMsg::Mint {
                        denom: denom.clone(),
                        amount: sale_category.recipients[0].amount,
                        recipient: env.contract.address.clone(),
                    },
                ))));

                coin(sale_category.recipients[0].amount.u128(), denom.to_string())
            } else {
                ensure!(
                    info.funds.len() == 1,
                    ContractError::Payment(PaymentError::MultipleDenoms {})
                );

                let launch_balance = info.funds[0].clone();

                ensure!(
                    sale_category.recipients[0].amount == launch_balance.amount,
                    ContractError::InvalidFunds {}
                );

                launch_balance
            };

            messages.push(SubMsg::reply_on_success(
                CosmosMsg::Wasm(wasm_execute(
                    config.pilot.pilot_contract,
                    &kujira_pilot::ExecuteMsg::Create {
                        sale: launch.pilot.clone().unwrap().sale,
                        orca: launch.pilot.clone().unwrap().orca,
                    },
                    vec![pilot_config.deposit, sale_funds],
                )?),
                ReplyTypes::Create as u64,
            ));

            REPLY.save(
                deps.storage,
                &ReplyInfo {
                    reply_type: ReplyTypes::Create,
                    idx,
                },
            )?;

            launch.status = LaunchStatus::InProgress;
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "pilot_start")
                .add_attribute("idx", idx)
                .add_submessages(messages))
        }
        ExecuteMsg::PilotExecute { idx } => {
            // Executes the pilot sale and sets the status to completed
            let mut launch = Launch::load(deps.storage, idx)?;

            ensure!(
                launch.status == LaunchStatus::InProgress,
                ContractError::Unauthorized {}
            );

            let execute = CosmosMsg::Wasm(wasm_execute(
                config.pilot.pilot_contract,
                &kujira_pilot::ExecuteMsg::Execute {
                    idx: launch.pilot.clone().unwrap().idx.unwrap(),
                },
                vec![],
            )?);

            launch.status = LaunchStatus::Completed;
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "pilot_execute")
                .add_attribute("idx", idx)
                .add_message(execute))
        }
        ExecuteMsg::PilotRetract { idx } => {
            // Retracts the pilot sale and sets the status to completed
            // Changes the denom admin back to the owner if the token is managed
            let mut launch = Launch::load(deps.storage, idx)?;
            launch.is_owner(&info.sender)?;

            ensure!(
                launch.status.clone() == LaunchStatus::InProgress,
                ContractError::Unauthorized {}
            );

            let mut messages = vec![];

            messages.push(CosmosMsg::Wasm(wasm_execute(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::ExecuteMsg::Retract {
                    idx: launch.clone().pilot.unwrap().idx.unwrap(),
                },
                vec![],
            )?));

            if launch.clone().token.unwrap().is_managed {
                messages.push(CosmosMsg::Custom(KujiraMsg::Denom(DenomMsg::ChangeAdmin {
                    denom: launch.clone().token.unwrap().denom,
                    address: launch.owner.clone(),
                })));
            }

            let categories = launch.clone().tokenomics.unwrap().categories;

            let sale_category = categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Sale)
                .unwrap();

            let pilot_config: kujira_pilot::ConfigResponse = deps.querier.query_wasm_smart(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::QueryMsg::Config {},
            )?;

            let mut amount = NativeBalance::default();
            amount.add_assign(coin(
                sale_category.recipients[0].amount.u128(),
                launch.clone().token.unwrap().denom.to_string(),
            ));
            amount.add_assign(pilot_config.deposit);
            amount.normalize();

            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: launch.owner.to_string(),
                amount: amount.into_vec(),
            }));

            launch.status = LaunchStatus::Completed;
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "pilot_retract")
                .add_attribute("idx", idx)
                .add_messages(messages))
        }
        ExecuteMsg::PostLaunch { idx } => {
            // Executes the post launch actions and sets the status to completed
            // Sets up the vesting schedules
            // Creates the FIN and BOW contracts
            // Changes the denom admin to the specified address
            let mut launch = Launch::load(deps.storage, idx)?;
            launch.is_owner(&info.sender)?;

            // InProgress is allowed for when Execute is called on the Pilot Contract directly
            ensure!(
                (launch.status == LaunchStatus::InProgress
                    || launch.status == LaunchStatus::Completed),
                ContractError::Unauthorized {}
            );

            //Ensure amount deposited is correct for non managed tokens
            if !launch.clone().token.unwrap().is_managed {
                let mut non_managed_amount = Uint128::zero();
                for category in launch.clone().tokenomics.unwrap().categories {
                    if category.category_type != CategoryTypes::Sale {
                        for recipient in category.recipients {
                            non_managed_amount = non_managed_amount.add(recipient.amount);
                        }
                    }
                }
                let mut balances = NativeBalance(info.funds);
                balances = balances.sub(config.token.denom_fee.clone())?;
                let balances = balances.into_vec();
                ensure!(balances.len() == 1, ContractError::InvalidFunds {});
                ensure!(
                    non_managed_amount == balances[0].amount,
                    ContractError::InvalidDeposit(
                        non_managed_amount.to_string(),
                        balances[0].amount.to_string()
                    )
                );
            } else {
                ensure!(info.funds.len() == 1, ContractError::InvalidFunds {});
                ensure!(
                    info.funds[0] == config.token.denom_fee,
                    ContractError::InvalidDeposit(
                        config.token.denom_fee.amount.to_string(),
                        info.funds[0].amount.to_string()
                    )
                );
            }

            let pilot_config: kujira_pilot::ConfigResponse = deps.querier.query_wasm_smart(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::QueryMsg::Config {},
            )?;

            // Only allow filled sales to execute this message
            let pilot_sale: StdResult<kujira_pilot::SaleResponse> = deps.querier.query_wasm_smart(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::QueryMsg::Sale {
                    idx: launch.clone().pilot.unwrap().idx.unwrap(),
                },
            );

            let (raise_total, raise_amount) = if let Status::Executed {
                at: _at,
                raise_total,
                raise_fee: _raise_fee,
                raise_amount,
            } = pilot_sale.unwrap().status
            {
                (raise_total, raise_amount)
            } else {
                return Err(ContractError::Unauthorized {});
            };

            let mut messages = vec![];

            let tokenomics = launch.clone().tokenomics.unwrap();
            let denom = launch.clone().token.unwrap().denom;
            let denom_symbol = launch.clone().token.unwrap().symbol;
            let bid_denom = launch.clone().pilot.unwrap().orca.bid_denom;
            let bid_denom_config = config
                .pilot
                .allowed_bid_denoms
                .iter()
                .find(|d| d.denom == bid_denom)
                .unwrap();

            // Setup categories of tokenomcs and vesting schedules
            for category in tokenomics.categories.clone() {
                if category.category_type == CategoryTypes::Standard {
                    for recipient in category.recipients {
                        if let Some(flows) = recipient.flows {
                            if launch.clone().token.unwrap().is_managed {
                                messages.push(CosmosMsg::Custom(KujiraMsg::Denom(
                                    DenomMsg::Mint {
                                        denom: denom.clone(),
                                        amount: recipient.amount,
                                        recipient: env.contract.address.clone(),
                                    },
                                )));
                            };
                            for flow in flows {
                                let mut flows_amount = 0u128;
                                for schedule in flow.clone().schedules {
                                    flows_amount = flows_amount.add(schedule.amount.u128());
                                }
                                messages.push(CosmosMsg::Wasm(wasm_execute(
                                    &config.flows.flows_contract,
                                    &fuzion_flows::ExecuteMsg::CreateFlows {
                                        flow_list: vec![flow],
                                    },
                                    coins(flows_amount, denom.to_string()),
                                )?));
                            }
                        } else if launch.clone().token.unwrap().is_managed {
                            messages.push(CosmosMsg::Custom(KujiraMsg::Denom(DenomMsg::Mint {
                                denom: denom.clone(),
                                amount: recipient.amount,
                                recipient: recipient.address.unwrap(),
                            })));
                        } else {
                            messages.push(CosmosMsg::Bank(BankMsg::Send {
                                amount: coins(recipient.amount.u128(), denom.clone().to_string()),
                                to_address: recipient.address.unwrap().to_string(),
                            }));
                        }
                    }
                }
            }

            // Setup FIN Pair Contract
            let CodeInfoResponse { checksum, .. } =
                deps.querier.query_wasm_code_info(config.fin.code_id)?;

            let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;

            let fin_salt = Binary::from(format!("keiko_fin_{idx}").as_bytes());
            let fin_address = deps
                .api
                .addr_humanize(&instantiate2_address(&checksum, &creator, &fin_salt).unwrap())?;

            // get the Sale Tokenomics category
            let sale_category = tokenomics
                .categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Sale)
                .unwrap();

            let average_price_of_launch =
                Decimal::from_ratio(raise_amount, sale_category.recipients[0].amount);

            let price_precision_decimals = if average_price_of_launch.lt(&Decimal::one()) {
                let num_str = average_price_of_launch.to_string();
                let parts: Vec<&str> = num_str.split('.').collect();
                parts[1].chars().take_while(|&c| c == '0').count() + 4
            } else if average_price_of_launch.ge(&Decimal::one())
                && average_price_of_launch.lt(&Decimal::from_str("1000.0")?)
            {
                3
            } else {
                2
            };

            let fin = CosmosMsg::Wasm(WasmMsg::Instantiate2 {
                admin: Some(config.fin.admin.clone().to_string()),
                code_id: config.fin.code_id,
                msg: to_json_binary(&kujira_fin::InstantiateMsg {
                    owner: env.contract.address.clone(),
                    denoms: [
                        cw20::Denom::Native(denom.to_string()),
                        cw20::Denom::Native(bid_denom.to_string()),
                    ],
                    decimal_delta: Some(
                        (launch.clone().token.unwrap().decimals - bid_denom_config.decimals) as i8,
                    ),
                    price_precision: Precision::DecimalPlaces(price_precision_decimals as u8),
                    fee_maker: config.fin.fee_maker,
                    fee_taker: config.fin.fee_taker,
                    fee_address: config.fin.fee_address,
                })?,
                funds: vec![],
                label: format!("FIN {}-{}", denom_symbol, bid_denom_config.symbol),
                salt: fin_salt,
            });

            messages.push(fin);

            // Setup BOW Market Maker Contract
            let CodeInfoResponse { checksum, .. } =
                deps.querier.query_wasm_code_info(config.bow.code_id)?;

            let bow_salt = Binary::from(format!("keiko_bow_{idx}").as_bytes());
            let bow_address = deps
                .api
                .addr_humanize(&instantiate2_address(&checksum, &creator, &bow_salt).unwrap())?;

            let bow = CosmosMsg::Wasm(WasmMsg::Instantiate2 {
                admin: Some(config.bow.admin.clone().to_string()),
                code_id: config.bow.code_id,
                msg: to_json_binary(&kujira::bow::market_maker::InstantiateMsg {
                    owner: config.bow.owner.clone(),
                    fin_contract: fin_address.clone(),
                    intervals: config.bow.intervals,
                    fee: config.bow.fee,
                    amp: config.bow.amp,
                })?,
                funds: vec![config.token.denom_fee.clone()],
                label: format!("Bow: {}-{}", denom_symbol, bid_denom_config.symbol),
                salt: bow_salt,
            });

            messages.push(bow);

            // Provide liquidity to BOW Market Maker
            // get the Liquidity Tokenomics category
            let lp_category = tokenomics
                .categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Liquidity)
                .unwrap();

            // calculate the LP to provide to the pool
            if launch.clone().token.unwrap().is_managed {
                messages.push(CosmosMsg::Custom(KujiraMsg::Denom(DenomMsg::Mint {
                    denom: denom.clone(),
                    amount: lp_category.recipients[0].amount,
                    recipient: env.contract.address.clone(),
                })));
            };
            let lp_denom_funds = coin(lp_category.recipients[0].amount.u128(), denom.to_string());

            let lp_stable_amount = raise_total.multiply_ratio(
                lp_category.recipients[0].amount.u128(),
                sale_category.recipients[0].amount.u128(),
            );
            let lp_stable_funds = coin(lp_stable_amount.u128(), bid_denom.to_string());

            let mut lp_funds = NativeBalance(vec![lp_denom_funds.clone(), lp_stable_funds]);
            lp_funds.normalize();
            let lp_funds = lp_funds.into_vec();
            let liquidity = CosmosMsg::Wasm(wasm_execute(
                bow_address.clone(),
                &kujira::bow::market_maker::execute::ExecuteMsg::Deposit {
                    max_slippage: None,
                    callback: Some(to_json_binary(&CallbackType::BowCallback { idx })?.into()),
                },
                lp_funds,
            )?);

            messages.push(liquidity);

            if launch.clone().token.unwrap().is_managed {
                let denom_admin =
                    if let Some(denom_admin) = launch.clone().token.unwrap().denom_admin {
                        denom_admin
                    } else {
                        config.token.default_admin
                    };
                // Change the denom admin
                messages.push(CosmosMsg::Custom(KujiraMsg::Denom(DenomMsg::ChangeAdmin {
                    denom,
                    address: denom_admin,
                })));

                // Register the token in Fuzion Products
                let managed_token = launch.clone().token.unwrap();
                let logo_uris: Option<LogoURIs> = Some(LogoURIs {
                    png: managed_token.png_url,
                    svg: managed_token.svg_url,
                });
                let asset = AssetList {
                    chain_name: "kujira".to_string(),
                    assets: vec![Asset {
                        description: Some(format!("{} Token", managed_token.symbol).to_string()),
                        denom_units: vec![
                            DenomUnit {
                                denom: managed_token.denom.to_string(),
                                exponent: 0,
                            },
                            DenomUnit {
                                denom: managed_token.symbol.to_string().to_lowercase(),
                                exponent: managed_token.decimals as u16,
                            },
                        ],
                        base: managed_token.denom.to_string(),
                        name: managed_token.symbol.to_string(),
                        display: managed_token.symbol.to_string().to_lowercase(),
                        symbol: managed_token.symbol.to_string(),
                        coingecko_id: None,
                        logo_uris,
                    }],
                };

                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.token.utilities_contract.to_string(),
                    msg: to_json_binary(&fuzion_utilities::ExecuteMsg::UploadAsset { asset })?,
                    funds: vec![],
                }));

                let coin = Coin {
                    denom: managed_token.denom.to_string(),
                    amount: Uint128::from(0_u128),
                };

                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.token.utilities_contract.to_string(),
                    msg: to_json_binary(&fuzion_utilities::ExecuteMsg::UploadTotalSupply {
                        total_supply_list: vec![coin.clone()],
                    })?,
                    funds: vec![],
                }));

                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.token.utilities_contract.to_string(),
                    msg: to_json_binary(&fuzion_utilities::ExecuteMsg::UploadCuratedDenoms {
                        curated_denom_list: vec![coin],
                    })?,
                    funds: vec![],
                }));
            }

            if !pilot_config.deposit.amount.is_zero() {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: launch.owner.to_string(),
                    amount: vec![pilot_config.deposit],
                }));
            }

            let beneficiary_funds = coin(
                raise_amount.u128().sub(lp_stable_amount.u128()),
                bid_denom.to_string(),
            );

            if !beneficiary_funds.amount.is_zero() {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: launch.clone().pilot.unwrap().beneficiary.to_string(),
                    amount: vec![beneficiary_funds],
                }));
            }

            launch.fin = Some(Fin {
                contract_address: Some(fin_address),
            });

            launch.bow = Some(Bow {
                contract_address: Some(bow_address),
            });

            launch.status = LaunchStatus::Completed;
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "post_execute")
                .add_attribute("idx", idx)
                .add_messages(messages))
        }
        ExecuteMsg::Update { launch } => {
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});
            let _ = Launch::load(deps.storage, launch.clone().idx)?;
            launch.save(deps.storage)?;
            Ok(Response::default().add_attribute("action", "update"))
        }
        ExecuteMsg::LaunchFin { idx } => {
            let launch = Launch::load(deps.storage, idx)?;
            ensure!(
                info.sender == config.owner
                    || info.sender == config.fin.owner
                    || info.sender == launch.owner,
                ContractError::Unauthorized {}
            );

            ensure!(
                (launch.status == LaunchStatus::Completed
                    && launch.fin.is_some()
                    && launch.bow.is_some()),
                ContractError::Unauthorized {}
            );

            let mut messages = vec![CosmosMsg::Wasm(wasm_execute(
                launch.clone().fin.unwrap().contract_address.unwrap(),
                &kujira_fin::ExecuteMsg::Launch {},
                vec![],
            )?)];

            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: launch.fin.unwrap().contract_address.unwrap().to_string(),
                msg: to_json_binary(&kujira_fin::ExecuteMsg::UpdateConfig {
                    owner: Some(config.fin.owner.clone()),
                    price_precision: None,
                    fee_taker: None,
                    fee_maker: None,
                })?,
                funds: vec![],
            }));

            Ok(Response::default()
                .add_attribute("action", "LaunchFin")
                .add_messages(messages))
        }
        ExecuteMsg::SetContractAdmin { contract, admin } => {
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});
            let messages = vec![CosmosMsg::Wasm(WasmMsg::UpdateAdmin {
                contract_addr: contract.to_string(),
                admin: admin.to_string(),
            })];

            Ok(Response::default()
                .add_attribute("action", "SetContractAdmin")
                .add_messages(messages))
        }
        ExecuteMsg::ExecuteContract { contract, msg } => {
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});

            let messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.to_string(),
                msg,
                funds: vec![],
            })];

            Ok(Response::default()
                .add_attribute("action", "ExecuteContract")
                .add_messages(messages))
        }
        ExecuteMsg::UpdateDescription { idx, description } => {
            let mut launch = Launch::load(deps.storage, idx)?;
            ensure!(
                (info.sender == config.owner || info.sender == launch.owner)
                    && launch.status == LaunchStatus::InProgress,
                ContractError::Unauthorized {}
            );
            let mut pilot = launch.clone().pilot.unwrap();
            pilot.sale.description = description.clone();
            launch.pilot = Some(pilot.clone());
            launch.save(deps.storage)?;
            Ok(Response::default()
                .add_attribute("action", "update")
                .add_message(CosmosMsg::Wasm(wasm_execute(
                    config.pilot.pilot_contract.clone(),
                    &kujira_pilot::ExecuteMsg::UpdateSaleDescription {
                        idx: pilot.idx.unwrap(),
                        description,
                    },
                    vec![],
                )?)))
        }
        ExecuteMsg::Callback(msg) => {
            // Executes the callback from the BOW Market Maker
            // Sends the LP tokens to the beneficiary
            let cb_msg = msg.deserialize_callback()?;
            let mut messages = vec![];
            match cb_msg {
                CallbackType::BowCallback { idx } => {
                    let launch = Launch::load(deps.storage, idx)?;
                    ensure!(
                        info.sender == launch.clone().bow.unwrap().contract_address.unwrap(),
                        ContractError::Unauthorized {}
                    );
                    ensure!(info.funds.len() == 1, ContractError::LPTokensNotReceived {});

                    let lp_flow = FlowCreate {
                        flow_type: FlowType::Vesting,
                        taker: launch.clone().pilot.unwrap().beneficiary,
                        denom: info.funds[0].denom.clone(),
                        genesis_time: env.block.time.seconds(),
                        identifier: None,
                        schedules: vec![FlowSchedule {
                            start_time: env.block.time.seconds(),
                            end_time: env.block.time.seconds()
                                + config.tokenomics.default_lp_vest_duration,
                            amount: info.funds[0].amount,
                            cliff_end_time: env.block.time.seconds()
                                + config.tokenomics.default_lp_vest_cliff,
                        }],
                    };

                    messages.push(CosmosMsg::Wasm(wasm_execute(
                        &config.flows.flows_contract,
                        &fuzion_flows::ExecuteMsg::CreateFlows {
                            flow_list: vec![lp_flow],
                        },
                        info.funds.clone(),
                    )?));

                    // Register the LP token in Fuzion Products
                    let bid_denom = launch.clone().pilot.unwrap().orca.bid_denom;
                    let bid_denom_config = config
                        .pilot
                        .allowed_bid_denoms
                        .iter()
                        .find(|d| d.denom == bid_denom)
                        .unwrap();
                    let managed_token = launch.clone().token.unwrap();
                    let logo_uris: Option<LogoURIs> = Some(LogoURIs {
                        png: managed_token.png_url,
                        svg: managed_token.svg_url,
                    });
                    let lp_symbol = format!(
                        "LP {}-{}",
                        managed_token.symbol.to_uppercase(),
                        bid_denom_config.symbol.to_uppercase()
                    );
                    let lp_display = format!(
                        "{}-{}-ulp",
                        managed_token.symbol.to_lowercase(),
                        bid_denom_config.symbol.to_lowercase()
                    );
                    let asset = AssetList {
                        chain_name: "kujira".to_string(),
                        assets: vec![Asset {
                            description: Some(
                                format!(
                                    "The LP token for the {}-{} pair",
                                    managed_token.symbol.to_uppercase(),
                                    bid_denom_config.symbol.to_uppercase()
                                )
                                .to_string(),
                            ),
                            denom_units: vec![
                                DenomUnit {
                                    denom: info.funds[0].denom.to_string(),
                                    exponent: 0,
                                },
                                DenomUnit {
                                    denom: lp_display.clone(),
                                    exponent: 6,
                                },
                            ],
                            base: info.funds[0].denom.to_string(),
                            name: lp_symbol.clone(),
                            display: lp_display,
                            symbol: lp_symbol,
                            coingecko_id: None,
                            logo_uris,
                        }],
                    };

                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: config.token.utilities_contract.to_string(),
                        msg: to_json_binary(&fuzion_utilities::ExecuteMsg::UploadAsset { asset })?,
                        funds: vec![],
                    }));

                    let coin = Coin {
                        denom: info.funds[0].denom.to_string(),
                        amount: Uint128::from(0_u128),
                    };

                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: config.token.utilities_contract.to_string(),
                        msg: to_json_binary(&fuzion_utilities::ExecuteMsg::UploadTotalSupply {
                            total_supply_list: vec![coin.clone()],
                        })?,
                        funds: vec![],
                    }));

                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: config.token.utilities_contract.to_string(),
                        msg: to_json_binary(&fuzion_utilities::ExecuteMsg::UploadCuratedDenoms {
                            curated_denom_list: vec![coin],
                        })?,
                        funds: vec![],
                    }));
                }
            }

            Ok(Response::default()
                .add_attribute("action", "callback")
                .add_messages(messages))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<KujiraQuery>, _: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Launch { idx } => {
            let launch = Launch::load(deps.storage, idx)?;
            to_json_binary(&launch)
        }
        QueryMsg::LaunchByPilotIdx { idx } => {
            let launches = launch()
                .idx
                .pilot_idx
                .prefix(idx.u128())
                .range(deps.storage, None, None, Order::Descending)
                .take(1)
                .map(|x| x.map(|y| y.1))
                .collect::<StdResult<Vec<Launch>>>()?;
            to_json_binary(&launches[0])
        }
        QueryMsg::Launches { start_after, limit } => {
            let launches = launch()
                .range(
                    deps.storage,
                    None,
                    start_after.map(|x| Bound::exclusive(x.u128())),
                    Order::Descending,
                )
                .take(limit.unwrap_or(10) as usize)
                .map(|x| x.map(|y| y.1))
                .collect::<StdResult<Vec<Launch>>>()?;
            to_json_binary(&launches)
        }
        QueryMsg::LaunchesByOwner {
            owner,
            start_after,
            limit,
        } => {
            let launches = launch()
                .idx
                .owner
                .prefix(owner.to_string())
                .range(
                    deps.storage,
                    None,
                    start_after.map(|x| Bound::exclusive(x.u128())),
                    Order::Descending,
                )
                .take(limit.unwrap_or(10) as usize)
                .map(|x| x.map(|y| y.1))
                .collect::<StdResult<Vec<Launch>>>()?;
            to_json_binary(&launches)
        }
        QueryMsg::LaunchesByStatus {
            status,
            start_after,
            limit,
        } => {
            let launches = launch()
                .idx
                .status
                .prefix(status.to_string())
                .range(
                    deps.storage,
                    None,
                    start_after.map(|x| Bound::exclusive(x.u128())),
                    Order::Descending,
                )
                .take(limit.unwrap_or(10) as usize)
                .map(|x| x.map(|y| y.1))
                .collect::<StdResult<Vec<Launch>>>()?;
            to_json_binary(&launches)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut<KujiraQuery>,
    _env: Env,
    msg: Reply,
) -> Result<Response<KujiraMsg>, ContractError> {
    let storage = deps.storage;
    let reply_info = REPLY.load(storage)?;
    let data = msg.result.into_result().map_err(StdError::generic_err)?;

    match msg.id {
        1 => {
            let mut launch = Launch::load(storage, reply_info.idx)?;

            let attribute = data
                .events
                .iter()
                .flat_map(|e| e.attributes.clone())
                .find(|a| a.key == "sale")
                .unwrap();

            let mut pilot = launch.pilot.unwrap();
            pilot.idx = Some(Uint128::from_str(&attribute.value)?);
            launch.pilot = Some(pilot);
            launch.save(storage)?;

            Ok(Response::new().add_attribute("create_reply_response_ok", msg.id.to_string()))
        }
        id => Err(ContractError::UnknownReplyId(id.to_string())),
    }
}
