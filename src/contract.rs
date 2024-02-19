use std::ops::AddAssign;
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, coins, ensure, instantiate2_address, to_json_binary, wasm_execute, BankMsg, Binary,
    CodeInfoResponse, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response, StdError,
    StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use cw_utils::{NativeBalance, PaymentError};
use fuzion_flows::FlowCreate;
use kujira::{DenomMsg, KujiraMsg, KujiraQuery, Precision};
use kujira_pilot::Status;

use crate::launch::Launch;
use crate::msg::{
    CategoryTypes, Config, LaunchStatus, Pilot, ReplyInfo, ReplyTypes, Token, Tokenomics,
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
        ExecuteMsg::Create {} => {
            // Sets up the launch and takes the deposit
            ensure!(
                info.funds.len() == 1 && info.funds[0] == config.pilot.deposit,
                ContractError::Payment(PaymentError::MissingDenom(config.pilot.deposit.denom))
            );

            let launch = Launch::new(deps.storage, info.sender.clone(), info.funds[0].clone());
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

            ensure!(launch.token.is_none(), ContractError::TokenAlreadyExists {});

            let mut messages: Vec<KujiraMsg> = vec![];

            let launch_denom = if create {
                ensure!(
                    info.funds.len() == 1,
                    ContractError::Payment(PaymentError::MultipleDenoms {})
                );

                ensure!(
                    info.funds[0] == config.token.denom_fee,
                    ContractError::InvalidFunds {}
                );

                ensure!(denom.is_none(), ContractError::DenomNotAllowed {});
                ensure!(decimals.is_none(), ContractError::DecimalsDefaulted {});

                let contract_address = env.contract.address.to_string();
                let symbol_lower = symbol.to_lowercase();

                // mints a new token
                let kujira_denom =
                    kujira::Denom::from(format!("factory/{contract_address}/u{symbol_lower}"));
                messages.push(KujiraMsg::Denom(DenomMsg::Create {
                    subdenom: kujira_denom.clone(),
                }));
                kujira_denom
            } else {
                denom.unwrap()
            };

            let launch_token = Token {
                denom: launch_denom.clone(),
                is_managed: create,
                symbol: symbol.clone(),
                decimals: if create { 6 } else { decimals.unwrap() },
                denom_admin,
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

            ensure!(
                liquidity_category.into_iter().next().unwrap().recipients[0]
                    .amount
                    .lt(&sale_category.into_iter().next().unwrap().recipients[0].amount),
                ContractError::LiquidityAmountSaleAmount {}
            );

            // ensure that each category has a recipient
            for category in categories.clone() {
                for recipient in category.recipients {
                    if recipient.address.is_none() {
                        return Err(ContractError::RecipientRequired(category.label));
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

            ensure!(
                config.pilot.allowed_bid_denoms.contains(&orca.bid_denom),
                ContractError::InvalidBidDenom {}
            );

            let categories = launch.clone().tokenomics.unwrap().categories;

            let sale_category = categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Sale)
                .unwrap();

            // ensure the minimum raise amount can be met
            let launch_min_raise_amount = sale_category.recipients[0].amount
                * (sale.price
                    * (Uint128::new(1u128)
                        - (Uint128::new(orca.max_slot as u128) * orca.premium_rate_per_slot)));

            ensure!(
                launch_min_raise_amount > config.pilot.min_raise_amount,
                ContractError::InvalidRaiseAmount(config.pilot.min_raise_amount.to_string())
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

            ensure!(
                launch.token.clone().is_some()
                    && launch.tokenomics.clone().is_some()
                    && launch.pilot.clone().is_some()
                    && launch.status == LaunchStatus::Planned
                    && launch.pilot.clone().unwrap().idx.is_none(),
                ContractError::Unauthorized {}
            );

            let categories = launch.clone().tokenomics.unwrap().categories;

            let sale_category = categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Sale)
                .unwrap();

            let mut messages = vec![];
            let denom = launch.clone().token.unwrap().denom;

            let sale_coin = if launch.clone().token.unwrap().is_managed {
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
                    vec![config.pilot.deposit, sale_coin],
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
                config.pilot.pilot_contract,
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

            let mut amount = NativeBalance::default();
            amount.add_assign(coin(
                sale_category.recipients[0].amount.u128(),
                launch.clone().token.unwrap().denom.to_string(),
            ));
            amount.add_assign(config.pilot.deposit);
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

            ensure!(
                (launch.status == LaunchStatus::InProgress
                    || launch.status == LaunchStatus::Completed),
                ContractError::Unauthorized {}
            );

            // Only allow filled sales to execute this message
            let pilot_sale: StdResult<kujira_pilot::SaleResponse> = deps.querier.query_wasm_smart(
                config.pilot.pilot_contract.clone(),
                &kujira_pilot::QueryMsg::Sale {
                    idx: launch.clone().pilot.unwrap().idx.unwrap(),
                },
            );

            let raise_total = if let Status::Executed {
                at: _at,
                raise_total,
                raise_fee: _raise_fee,
            } = pilot_sale.unwrap().status
            {
                raise_total
            } else {
                return Err(ContractError::Unauthorized {});
            };

            let mut messages = vec![];

            let tokenomics = launch.clone().tokenomics.unwrap();
            let denom = launch.clone().token.unwrap().denom;
            let bid_denom = launch.clone().pilot.unwrap().orca.bid_denom;

            // Setup categories of tokenomcs and vesting schedules
            for category in tokenomics.categories.clone() {
                if category.category_type == CategoryTypes::Standard {
                    for recipient in category.recipients {
                        if let Some(schedules) = recipient.schedules {
                            messages.push(CosmosMsg::Custom(KujiraMsg::Denom(DenomMsg::Mint {
                                denom: denom.clone(),
                                amount: recipient.amount,
                                recipient: env.contract.address.clone(),
                            })));
                            let flow = FlowCreate {
                                flow_type: fuzion_flows::FlowType::Vesting,
                                taker: recipient.address.unwrap(),
                                denom: denom.clone().to_string(),
                                identifier: None,
                                genesis_time: env.block.time.seconds(),
                                schedules,
                            };
                            messages.push(CosmosMsg::Wasm(wasm_execute(
                                &config.flows.flows_contract,
                                &fuzion_flows::ExecuteMsg::CreateFlows {
                                    flow_list: vec![flow],
                                },
                                coins(recipient.amount.u128(), denom.to_string()),
                            )?));
                        } else {
                            messages.push(CosmosMsg::Custom(KujiraMsg::Denom(DenomMsg::Mint {
                                denom: denom.clone(),
                                amount: recipient.amount,
                                recipient: recipient.address.unwrap(),
                            })));
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

            let fin = CosmosMsg::Wasm(WasmMsg::Instantiate2 {
                admin: Some(env.contract.address.to_string()),
                code_id: config.fin.code_id,
                msg: to_json_binary(&kujira::fin::InstantiateMsg {
                    owner: env.contract.address.clone(),
                    denoms: [
                        cw20::Denom::Native(denom.to_string()),
                        cw20::Denom::Native(bid_denom.to_string()),
                    ],
                    decimal_delta: Some(5),
                    price_precision: Precision::DecimalPlaces(5),
                    fee_maker: config.fin.fee_maker,
                    fee_taker: config.fin.fee_taker,
                })?,
                funds: vec![],
                label: format!("FIN {}-{}", denom, bid_denom),
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
                admin: Some(env.contract.address.to_string()),
                code_id: config.bow.code_id,
                msg: to_json_binary(&kujira::bow::market_maker::InstantiateMsg {
                    owner: env.contract.address.clone(),
                    fin_contract: fin_address,
                    intervals: config.bow.intervals,
                    fee: config.bow.fee,
                    amp: config.bow.amp,
                })?,
                funds: vec![],
                label: format!("Bow: {}-{}", denom, bid_denom),
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

            // get the Sale Tokenomics category
            let sale_category = tokenomics
                .categories
                .iter()
                .find(|c| c.category_type == CategoryTypes::Sale)
                .unwrap();

            // calculate the LP to provide to the pool
            let denom_lp_coin = coin(lp_category.recipients[0].amount.u128(), denom.to_string());
            let stable_amount = raise_total.multiply_ratio(
                lp_category.recipients[0].amount.u128(),
                sale_category.recipients[0].amount.u128(),
            );
            let stable_lp_coin = coin(stable_amount.u128(), bid_denom.to_string());

            let liquidity = CosmosMsg::Wasm(wasm_execute(
                bow_address.clone(),
                &kujira::bow::market_maker::execute::ExecuteMsg::Deposit {
                    max_slippage: None,
                    callback: None,
                },
                vec![denom_lp_coin, stable_lp_coin],
            )?);

            messages.push(liquidity);

            // Change the denom admin
            messages.push(CosmosMsg::Custom(KujiraMsg::Denom(DenomMsg::ChangeAdmin {
                denom,
                address: launch
                    .clone()
                    .token
                    .unwrap()
                    .denom_admin
                    .unwrap_or_else(|| {
                        deps.api
                            .addr_validate(config.token.default_admin.as_str())
                            .unwrap()
                    }),
            })));

            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: launch.owner.to_string(),
                amount: vec![config.pilot.deposit],
            }));

            launch.status = LaunchStatus::Completed;
            launch.save(deps.storage)?;

            Ok(Response::default()
                .add_attribute("action", "post_execute")
                .add_attribute("idx", idx)
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let reply_info = REPLY.load(deps.storage)?;
    let data = msg.result.into_result().map_err(StdError::generic_err)?;

    match msg.id {
        1 => {
            let launch = Launch::load(deps.storage, reply_info.idx)?;

            let attribute = data
                .events
                .iter()
                .flat_map(|e| e.attributes.clone())
                .find(|a| a.key == "sale")
                .unwrap();

            launch.pilot.clone().unwrap().idx = Some(Uint128::from_str(&attribute.value)?);
            launch.save(deps.storage)?;

            Ok(Response::new().add_attribute("create_reply_response_ok", msg.id.to_string()))
        }
        id => Err(ContractError::UnknownReplyId(id.to_string())),
    }
}
