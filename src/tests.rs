use std::str::FromStr;

use crate::{
    contract::{execute, instantiate, query, reply},
    launch::Launch,
    msg::{
        BidDenoms, BowConfig, CategoryTypes, FinConfig, FlowsConfig, LaunchStatus, PilotConfig,
        TokenConfig, Tokenomics, TokenomicsCategories, TokenomicsConfig, TokenomicsRecipient,
    },
};

use super::*;
use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, Binary, Coin, Decimal, Decimal256, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Timestamp, Uint128,
};
use cw_multi_test::{ContractWrapper, Executor};
use cw_utils::PaymentError;
use kujira::{Denom, KujiraMsg, KujiraQuery};
use kujira_pilot::{CreateOrca, CreateSale};
use kujira_rs_testing::mock::{mock_app, CustomApp};

pub fn bow_execute(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: kujira_bow::market_maker::ExecuteMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    Ok(Response::default())
}

pub fn bow_instantiate(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: kujira_bow::market_maker::InstantiateMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    Ok(Response::default())
}

pub fn bow_query(_deps: Deps<KujiraQuery>, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    to_json_binary("")
}

pub fn fin_execute(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: kujira_fin::ExecuteMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    Ok(Response::default())
}

pub fn fin_instantiate(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: kujira_fin::InstantiateMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    Ok(Response::default())
}

pub fn fin_query(_deps: Deps<KujiraQuery>, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    to_json_binary("")
}

pub fn utilities_execute(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: fuzion_utilities::ExecuteMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    Ok(Response::default())
}

pub fn utilities_instantiate(
    _deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: fuzion_utilities::InstantiateMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    Ok(Response::default())
}

pub fn utilities_query(_deps: Deps<KujiraQuery>, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    to_json_binary("")
}

#[test]
fn launch_new_token() {
    let mut app: CustomApp = mock_app(vec![
        (
            Addr::unchecked("launcher"),
            [
                coin(1_000_000_000_000_000, "usk"),
                coin(1_000_000_000_000_000, "factory/kujira14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sl4e867/usned")
            ].to_vec()
        ),
        (
            Addr::unchecked("bidder"),
            coins(1_000_000_000_000_000, "bid"),
        ),
    ]);

    let contract = Box::new(ContractWrapper::new(execute, instantiate, query).with_reply(reply));
    let code_id = app.store_code(contract);

    let pilot_code_id = app.store_code(Box::new(
        ContractWrapper::new(
            kujira_pilot_testing::contract::execute,
            kujira_pilot_testing::contract::instantiate,
            kujira_pilot_testing::contract::query,
        )
        .with_reply(kujira_pilot_testing::contract::reply),
    ));
    let orca_code_id = app.store_code(Box::new(ContractWrapper::new(
        kujira_orca_queue::contract::execute,
        kujira_orca_queue::contract::instantiate,
        kujira_orca_queue::contract::query,
    )));
    let fin_code_id = app.store_code(Box::new(ContractWrapper::new(
        fin_execute,
        fin_instantiate,
        fin_query,
    )));
    let bow_code_id = app.store_code(Box::new(ContractWrapper::new(
        bow_execute,
        bow_instantiate,
        bow_query,
    )));
    let utilities_code_id = app.store_code(Box::new(ContractWrapper::new(
        utilities_execute,
        utilities_instantiate,
        utilities_query,
    )));

    let utilities_addr = app
        .instantiate_contract(
            utilities_code_id,
            Addr::unchecked("sender"),
            &fuzion_utilities::InstantiateMsg {
                admin: Some("utilities_admin".to_string()),
            },
            &[],
            "UTILITITES",
            None,
        )
        .unwrap();

    let pilot_addr = app
        .instantiate_contract(
            pilot_code_id,
            Addr::unchecked("sender"),
            &kujira_pilot::InstantiateMsg {
                owner: Addr::unchecked("owner"),
                deposit: Coin {
                    denom: "usk".to_string(),
                    amount: Uint128::from(1_000_000_000u128),
                },
                orca_code_id,
                sale_fee: Decimal::from_str("0.05").unwrap(),
                withdrawal_fee: Decimal::from_str("0.005").unwrap(),
            },
            &[],
            "KEIKO",
            None,
        )
        .unwrap();

    let keiko_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("sender"),
            &InstantiateMsg {
                owner: Addr::unchecked("owner"),
                token: TokenConfig {
                    denom_fee: Coin {
                        denom: "usk".to_string(),
                        amount: Uint128::from(10_000_000u128),
                    },
                    default_admin: Addr::unchecked("kujira10d07y265gmmuvt4z0w9aw880jnsr700jt23ame"),
                    utilities_contract: Addr::unchecked(utilities_addr),
                },
                tokenomics: TokenomicsConfig {
                    minimum_liquidity_one_side: Decimal::from_str("0.1").unwrap(),
                    default_lp_vest_cliff: 0,
                    default_lp_vest_duration: 60000,
                },
                pilot: PilotConfig {
                    pilot_contract: pilot_addr.clone(),
                    allowed_bid_denoms: vec![BidDenoms {
                        denom: Denom::from("bid"),
                        symbol: "bid".to_string(),
                        decimals: 6,
                    }],
                    min_raise_amount: Uint128::from(100_000_000_000u128),
                },
                flows: FlowsConfig {
                    flows_contract: Addr::unchecked("flows"),
                },
                fin: FinConfig {
                    code_id: fin_code_id,
                    owner: Addr::unchecked("owner"),
                    admin: Addr::unchecked("admin"),
                    fee_maker: Decimal256::from_str("0.00075").unwrap(),
                    fee_taker: Decimal256::from_str("0.0015").unwrap(),
                },
                bow: BowConfig {
                    code_id: bow_code_id,
                    owner: Addr::unchecked("owner"),
                    admin: Addr::unchecked("admin"),
                    intervals: vec![
                        Decimal::from_str("0.001").unwrap(),
                        Decimal::from_str("0.005").unwrap(),
                        Decimal::from_str("0.005").unwrap(),
                        Decimal::from_str("0.01").unwrap(),
                        Decimal::from_str("0.01").unwrap(),
                        Decimal::from_str("0.01").unwrap(),
                        Decimal::from_str("0.1").unwrap(),
                        Decimal::from_str("0.2").unwrap(),
                    ],
                    fee: Decimal::from_str("0.001").unwrap(),
                    amp: Decimal::from_str("1").unwrap(),
                },
            },
            &[],
            "KEIKO",
            None,
        )
        .unwrap();

    let err = app
        .execute_contract(
            Addr::unchecked("launcher"),
            keiko_addr.clone(),
            &ExecuteMsg::Create {
                terms_conditions_accepted: false,
            },
            &coins(1_000_000_000, "usk"),
        )
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();

    assert_eq!(err, ContractError::TermsConditionsAccepted {});

    let err = app
        .execute_contract(
            Addr::unchecked("launcher"),
            keiko_addr.clone(),
            &ExecuteMsg::Create {
                terms_conditions_accepted: true,
            },
            &coins(100_000_000, "usk"),
        )
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();

    assert_eq!(
        err,
        ContractError::Payment(PaymentError::MissingDenom("usk".to_string()))
    );

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &ExecuteMsg::Create {
            terms_conditions_accepted: true,
        },
        &coins(1_000_000_000, "usk"),
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.idx, Uint128::zero());
    assert_eq!(Some(true), launch.terms_conditions_accepted);
    assert_eq!(launch.status, LaunchStatus::Created);
    assert_eq!(launch.deposit, coin(1_000_000_000, "usk"));

    let keiko_balances = app
        .wrap()
        .query_all_balances(keiko_addr.clone().to_string())
        .unwrap();
    assert_eq!(keiko_balances, coins(1_000_000_000, "usk"));

    let err = app
        .execute_contract(
            Addr::unchecked("launcher"),
            keiko_addr.clone(),
            &&ExecuteMsg::Token {
                idx: Uint128::zero(),
                create: true,
                symbol: Some("SNED".to_string()),
                denom: None,
                decimals: None,
                denom_admin: None,
                png_url: Some("https://example.com/sned.png".to_string()),
                svg_url: Some("https://example.com/sned.svg".to_string()),
            },
            &coins(100_000_000, "usk"),
        )
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();

    assert_eq!(err, ContractError::InvalidFunds {});

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::Token {
            idx: Uint128::zero(),
            create: true,
            symbol: Some("SNED".to_string()),
            denom: None,
            decimals: None,
            denom_admin: None,
            png_url: Some("https://example.com/sned.png".to_string()),
            svg_url: Some("https://example.com/sned.svg".to_string()),
        },
        &coins(10_000_000, "usk"),
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert!(launch.token.is_some());
    let token_response = launch.token.unwrap();
    assert!(token_response.is_managed);
    assert_eq!(token_response.symbol, "SNED");
    assert_eq!(token_response.decimals, 6);
    assert_eq!(token_response.denom_admin, None);
    assert_eq!(
        token_response.png_url,
        Some("https://example.com/sned.png".to_string())
    );
    assert_eq!(
        token_response.svg_url,
        Some("https://example.com/sned.svg".to_string())
    );
    assert_eq!(launch.status, LaunchStatus::Created);

    let tokenomics = Tokenomics {
        categories: vec![
            TokenomicsCategories {
                label: "Sale".to_string(),
                category_type: CategoryTypes::Sale,
                recipients: vec![TokenomicsRecipient {
                    address: None,
                    amount: Uint128::from(1_000_000_000_000u128),
                    flows: None,
                }],
            },
            TokenomicsCategories {
                label: "Liquidity".to_string(),
                category_type: CategoryTypes::Liquidity,
                recipients: vec![TokenomicsRecipient {
                    address: None,
                    amount: Uint128::from(100_000_000_000u128),
                    flows: None,
                }],
            },
        ],
    };

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::Tokenomics {
            idx: launch.idx,
            categories: tokenomics.clone().categories,
        },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert!(launch.tokenomics.is_some());
    assert_eq!(launch.tokenomics.unwrap(), tokenomics);
    assert_eq!(launch.status, LaunchStatus::Created);

    let pilot_sale = CreateSale {
        title: "SNED".to_string(),
        description: "SNED Launch".to_string(),
        url: "https://example.com/sned".to_string(),
        beneficiary: Addr::unchecked("beneficiary"),
        price: Decimal::from_str("0.1").unwrap(),
        opens: Timestamp::from_seconds(app.block_info().time.seconds() + 100),
        closes: Timestamp::from_seconds(app.block_info().time.seconds() + 1000),
    };

    let create_orca = CreateOrca {
        bid_denom: Denom::from("bid".to_string()),
        max_slot: 10,
        premium_rate_per_slot: Decimal::from_str("0.05").unwrap(),
        bid_threshold: Uint128::from(1_000_000_000u128),
        waiting_period: 600,
    };

    let err = app
        .execute_contract(
            Addr::unchecked("launcher"),
            keiko_addr.clone(),
            &&ExecuteMsg::PilotSchedule {
                idx: launch.idx,
                sale: pilot_sale,
                orca: create_orca,
            },
            &[],
        )
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();

    assert_eq!(
        err,
        ContractError::InvalidRaiseAmount(
            Uint128::from(50_000_000_000u128).to_string(),
            Uint128::from(100_000_000_000u128).to_string()
        )
    );

    let pilot_sale = CreateSale {
        title: "SNED".to_string(),
        description: "SNED Launch".to_string(),
        url: "https://example.com/sned".to_string(),
        beneficiary: Addr::unchecked("beneficiary"),
        price: Decimal::from_str("1").unwrap(),
        opens: Timestamp::from_seconds(app.block_info().time.seconds() + 100),
        closes: Timestamp::from_seconds(app.block_info().time.seconds() + 1000),
    };

    let create_orca = CreateOrca {
        bid_denom: Denom::from("bid".to_string()),
        max_slot: 9,
        premium_rate_per_slot: Decimal::from_str("0.1").unwrap(),
        bid_threshold: Uint128::from(1_000_000_000u128),
        waiting_period: 600,
    };

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotSchedule {
            idx: launch.idx,
            sale: pilot_sale.clone(),
            orca: create_orca.clone(),
        },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert!(launch.pilot.is_some());
    let pilot = launch.pilot.unwrap();
    let mut pilot_sale_updated = pilot_sale.clone();
    pilot_sale_updated.beneficiary = keiko_addr.clone();
    assert_eq!(pilot.sale, pilot_sale_updated);
    assert_eq!(pilot.orca, create_orca);
    assert_eq!(pilot.beneficiary, pilot_sale.beneficiary);
    assert_eq!(launch.status, LaunchStatus::Planned);

    let mut new_block = app.block_info();
    new_block.time = Timestamp::from_seconds(new_block.time.seconds() + 100);
    app.set_block(new_block);

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotStart { idx: launch.idx },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.clone().pilot.unwrap().idx, Some(Uint128::zero()));
    assert_eq!(launch.status, LaunchStatus::InProgress);

    let pilot_sale: kujira_pilot::SaleResponse = app
        .wrap()
        .query_wasm_smart(
            pilot_addr.clone(),
            &kujira_pilot::QueryMsg::Sale {
                idx: launch.pilot.unwrap().idx.unwrap(),
            },
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked("bidder"),
        pilot_sale.orca_address.clone(),
        &&kujira_orca::ExecuteMsg::SubmitBid {
            premium_slot: 9,
            delegate: None,
            proof: None,
        },
        &[coin(600_000_000_000, "bid")],
    )
    .unwrap();

    let orca_bids: kujira_orca::BidsResponse = app
        .wrap()
        .query_wasm_smart(
            pilot_sale.orca_address.clone(),
            &kujira_orca::QueryMsg::BidsByUser {
                bidder: Addr::unchecked("bidder"),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(orca_bids.bids.len(), 1);

    let mut new_block = app.block_info();
    new_block.time = Timestamp::from_seconds(new_block.time.seconds() + 900);
    app.set_block(new_block);

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotExecute { idx: launch.idx },
        &[],
    )
    .unwrap_err();

    let mut new_block = app.block_info();
    new_block.time = Timestamp::from_seconds(new_block.time.seconds() + 1);
    app.set_block(new_block);

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotExecute { idx: launch.idx },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.status, LaunchStatus::Completed);

    let pilot_sale: kujira_pilot::SaleResponse = app
        .wrap()
        .query_wasm_smart(
            pilot_addr.clone(),
            &kujira_pilot::QueryMsg::Sale {
                idx: launch.pilot.unwrap().idx.unwrap(),
            },
        )
        .unwrap();

    let expected_status = kujira_pilot::Status::Executed {
        at: Timestamp::from_nanos(1571798420000000000),
        raise_total: Uint128::from(100_000_000_000u128),
        raise_fee: Uint128::from(4_999_999_999u128),
        raise_amount: Uint128::from(95_000_000_000u128),
    };

    assert_eq!(pilot_sale.status, expected_status);

    let _keiko_balances = app.wrap().query_all_balances(keiko_addr.clone()).unwrap();

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PostLaunch { idx: launch.idx },
        &[coin(10_000_000, "usk")],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.status, LaunchStatus::Completed);
    assert!(launch.fin.is_some());
    assert!(launch.bow.is_some());
}

#[test]
fn launch_own_token() {
    let mut app: CustomApp = mock_app(vec![
        (
            Addr::unchecked("launcher"),
            [
                coin(1_000_000_000_000_000, "usk"),
                coin(1_000_000_000_000_000, "snedown"),
            ]
            .to_vec(),
        ),
        (
            Addr::unchecked("bidder"),
            coins(1_000_000_000_000_000, "bid"),
        ),
    ]);
    let contract = Box::new(ContractWrapper::new(execute, instantiate, query).with_reply(reply));
    let code_id = app.store_code(contract);

    let pilot_code_id = app.store_code(Box::new(
        ContractWrapper::new(
            kujira_pilot_testing::contract::execute,
            kujira_pilot_testing::contract::instantiate,
            kujira_pilot_testing::contract::query,
        )
        .with_reply(kujira_pilot_testing::contract::reply),
    ));
    let orca_code_id = app.store_code(Box::new(ContractWrapper::new(
        kujira_orca_queue::contract::execute,
        kujira_orca_queue::contract::instantiate,
        kujira_orca_queue::contract::query,
    )));
    let fin_code_id = app.store_code(Box::new(ContractWrapper::new(
        fin_execute,
        fin_instantiate,
        fin_query,
    )));
    let bow_code_id = app.store_code(Box::new(ContractWrapper::new(
        bow_execute,
        bow_instantiate,
        bow_query,
    )));
    let utilities_code_id = app.store_code(Box::new(ContractWrapper::new(
        utilities_execute,
        utilities_instantiate,
        utilities_query,
    )));

    let utilities_addr = app
        .instantiate_contract(
            utilities_code_id,
            Addr::unchecked("sender"),
            &fuzion_utilities::InstantiateMsg {
                admin: Some("utilities_admin".to_string()),
            },
            &[],
            "UTILITITES",
            None,
        )
        .unwrap();

    let pilot_addr = app
        .instantiate_contract(
            pilot_code_id,
            Addr::unchecked("sender"),
            &kujira_pilot::InstantiateMsg {
                owner: Addr::unchecked("owner"),
                deposit: Coin {
                    denom: "usk".to_string(),
                    amount: Uint128::from(1_000_000_000u128),
                },
                orca_code_id,
                sale_fee: Decimal::from_str("0.05").unwrap(),
                withdrawal_fee: Decimal::from_str("0.005").unwrap(),
            },
            &[],
            "KEIKO",
            None,
        )
        .unwrap();

    let keiko_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("sender"),
            &InstantiateMsg {
                owner: Addr::unchecked("owner"),
                token: TokenConfig {
                    denom_fee: Coin {
                        denom: "usk".to_string(),
                        amount: Uint128::from(10_000_000u128),
                    },
                    default_admin: Addr::unchecked("governance"),
                    utilities_contract: Addr::unchecked(utilities_addr),
                },
                tokenomics: TokenomicsConfig {
                    minimum_liquidity_one_side: Decimal::from_str("0.1").unwrap(),
                    default_lp_vest_cliff: 0,
                    default_lp_vest_duration: 60000,
                },
                pilot: PilotConfig {
                    pilot_contract: pilot_addr.clone(),
                    allowed_bid_denoms: vec![BidDenoms {
                        denom: Denom::from("bid"),
                        symbol: "bid".to_string(),
                        decimals: 6,
                    }],
                    min_raise_amount: Uint128::from(100_000_000_000u128),
                },
                flows: FlowsConfig {
                    flows_contract: Addr::unchecked("flows"),
                },
                fin: FinConfig {
                    code_id: fin_code_id,
                    owner: Addr::unchecked("owner"),
                    admin: Addr::unchecked("admin"),
                    fee_maker: Decimal256::from_str("0.00075").unwrap(),
                    fee_taker: Decimal256::from_str("0.0015").unwrap(),
                },
                bow: BowConfig {
                    code_id: bow_code_id,
                    owner: Addr::unchecked("owner"),
                    admin: Addr::unchecked("admin"),
                    intervals: vec![
                        Decimal::from_str("0.001").unwrap(),
                        Decimal::from_str("0.005").unwrap(),
                        Decimal::from_str("0.005").unwrap(),
                        Decimal::from_str("0.01").unwrap(),
                        Decimal::from_str("0.01").unwrap(),
                        Decimal::from_str("0.01").unwrap(),
                        Decimal::from_str("0.1").unwrap(),
                        Decimal::from_str("0.2").unwrap(),
                    ],
                    fee: Decimal::from_str("0.001").unwrap(),
                    amp: Decimal::from_str("1").unwrap(),
                },
            },
            &[],
            "KEIKO",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &ExecuteMsg::Create {
            terms_conditions_accepted: true,
        },
        &coins(1_000_000_000, "usk"),
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.idx, Uint128::zero());
    assert_eq!(Some(true), launch.terms_conditions_accepted);
    assert_eq!(launch.status, LaunchStatus::Created);
    assert_eq!(launch.deposit, coin(1_000_000_000, "usk"));

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::Token {
            idx: Uint128::zero(),
            create: false,
            symbol: Some("SNED".to_string()),
            denom: Some(Denom::from("snedown")),
            decimals: Some(6),
            denom_admin: Some(Addr::unchecked(
                "kujira10d07y265gmmuvt4z0w9aw880jnsr700jt23ame",
            )),
            png_url: Some("https://example.com/sned.png".to_string()),
            svg_url: Some("https://example.com/sned.svg".to_string()),
        },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert!(launch.token.is_some());
    let token_response = launch.token.unwrap();
    assert!(!token_response.is_managed);
    assert_eq!(token_response.symbol, "SNED");
    assert_eq!(token_response.decimals, 6);
    assert_eq!(
        token_response.denom_admin,
        Some(Addr::unchecked(
            "kujira10d07y265gmmuvt4z0w9aw880jnsr700jt23ame"
        ))
    );
    assert_eq!(
        token_response.png_url,
        Some("https://example.com/sned.png".to_string())
    );
    assert_eq!(
        token_response.svg_url,
        Some("https://example.com/sned.svg".to_string())
    );
    assert_eq!(launch.status, LaunchStatus::Created);

    let tokenomics = Tokenomics {
        categories: vec![
            TokenomicsCategories {
                label: "Sale".to_string(),
                category_type: CategoryTypes::Sale,
                recipients: vec![TokenomicsRecipient {
                    address: None,
                    amount: Uint128::from(1_000_000_000_000u128),
                    flows: None,
                }],
            },
            TokenomicsCategories {
                label: "Liquidity".to_string(),
                category_type: CategoryTypes::Liquidity,
                recipients: vec![TokenomicsRecipient {
                    address: None,
                    amount: Uint128::from(100_000_000_000u128),
                    flows: None,
                }],
            },
        ],
    };

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::Tokenomics {
            idx: launch.idx,
            categories: tokenomics.clone().categories,
        },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert!(launch.tokenomics.is_some());
    assert_eq!(launch.tokenomics.unwrap(), tokenomics);
    assert_eq!(launch.status, LaunchStatus::Created);

    let pilot_sale = CreateSale {
        title: "SNED".to_string(),
        description: "SNED Launch".to_string(),
        url: "https://example.com/sned".to_string(),
        beneficiary: Addr::unchecked("beneficiary"),
        price: Decimal::from_str("1").unwrap(),
        opens: Timestamp::from_seconds(app.block_info().time.seconds() + 100),
        closes: Timestamp::from_seconds(app.block_info().time.seconds() + 1000),
    };

    let create_orca = CreateOrca {
        bid_denom: Denom::from("bid".to_string()),
        max_slot: 10,
        premium_rate_per_slot: Decimal::from_str("0.05").unwrap(),
        bid_threshold: Uint128::from(1_000_000_000u128),
        waiting_period: 600,
    };

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotSchedule {
            idx: launch.idx,
            sale: pilot_sale.clone(),
            orca: create_orca.clone(),
        },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert!(launch.pilot.is_some());
    let pilot = launch.pilot.unwrap();
    let mut pilot_sale_updated = pilot_sale.clone();
    pilot_sale_updated.beneficiary = keiko_addr.clone();
    assert_eq!(pilot.sale, pilot_sale_updated);
    assert_eq!(pilot.orca, create_orca);
    assert_eq!(pilot.beneficiary, pilot_sale.beneficiary);
    assert_eq!(launch.status, LaunchStatus::Planned);

    let mut new_block = app.block_info();
    new_block.time = Timestamp::from_seconds(new_block.time.seconds() + 100);
    app.set_block(new_block);

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotStart { idx: launch.idx },
        &coins(1_000_000_000_000, "snedown"),
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.pilot.unwrap().idx, Some(Uint128::zero()));
    assert_eq!(launch.status, LaunchStatus::InProgress);

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.clone().pilot.unwrap().idx, Some(Uint128::zero()));
    assert_eq!(launch.status, LaunchStatus::InProgress);

    let pilot_sale: kujira_pilot::SaleResponse = app
        .wrap()
        .query_wasm_smart(
            pilot_addr.clone(),
            &kujira_pilot::QueryMsg::Sale {
                idx: launch.pilot.unwrap().idx.unwrap(),
            },
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked("bidder"),
        pilot_sale.orca_address.clone(),
        &&kujira_orca::ExecuteMsg::SubmitBid {
            premium_slot: 9,
            delegate: None,
            proof: None,
        },
        &[coin(600_000_000_000, "bid")],
    )
    .unwrap();

    let orca_bids: kujira_orca::BidsResponse = app
        .wrap()
        .query_wasm_smart(
            pilot_sale.orca_address.clone(),
            &kujira_orca::QueryMsg::BidsByUser {
                bidder: Addr::unchecked("bidder"),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(orca_bids.bids.len(), 1);

    let mut new_block = app.block_info();
    new_block.time = Timestamp::from_seconds(new_block.time.seconds() + 900);
    app.set_block(new_block);

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotExecute { idx: launch.idx },
        &[],
    )
    .unwrap_err();

    let mut new_block = app.block_info();
    new_block.time = Timestamp::from_seconds(new_block.time.seconds() + 1);
    app.set_block(new_block);

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PilotExecute { idx: launch.idx },
        &[],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.status, LaunchStatus::Completed);

    let pilot_sale: kujira_pilot::SaleResponse = app
        .wrap()
        .query_wasm_smart(
            pilot_addr.clone(),
            &kujira_pilot::QueryMsg::Sale {
                idx: launch.pilot.unwrap().idx.unwrap(),
            },
        )
        .unwrap();

    let expected_status = kujira_pilot::Status::Executed {
        at: Timestamp::from_nanos(1571798420000000000),
        raise_total: Uint128::from(550_000_000_000u128),
        raise_fee: Uint128::from(27_499_999_999u128),
        raise_amount: Uint128::from(522_500_000_000u128),
    };

    assert_eq!(pilot_sale.status, expected_status);

    app.send_tokens(
        Addr::unchecked("bidder"),
        keiko_addr.clone(),
        &[coin(
            Uint128::from(100_000_000_000u128)
                .checked_sub(Uint128::from(4_999_999_999u128))
                .unwrap()
                .u128(),
            "bid",
        )],
    )
    .unwrap();

    app.send_tokens(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &[coin(100_000_000_000, "usk")],
    )
    .unwrap();

    app.send_tokens(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &[coin(100_000_000_000, "snedown")],
    )
    .unwrap();

    let _keiko_balances = app.wrap().query_all_balances(keiko_addr.clone()).unwrap();

    app.execute_contract(
        Addr::unchecked("launcher"),
        keiko_addr.clone(),
        &&ExecuteMsg::PostLaunch { idx: launch.idx },
        &[coin(100_000_000_000, "snedown"), coin(10_000_000, "usk")],
    )
    .unwrap();

    let launch: Launch = app
        .wrap()
        .query_wasm_smart(
            keiko_addr.clone(),
            &QueryMsg::Launch {
                idx: Uint128::zero(),
            },
        )
        .unwrap();

    assert_eq!(launch.status, LaunchStatus::Completed);
    assert!(launch.fin.is_some());
    assert!(launch.bow.is_some());
}
