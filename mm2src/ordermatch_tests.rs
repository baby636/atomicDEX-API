use super::*;
use crate::mm2::lp_network::P2PContext;
use crate::mm2::lp_ordermatch::new_protocol::PubkeyKeepAlive;
use coins::{MmCoin, TestCoin};
use common::rusqlite::Connection;
use common::{block_on,
             executor::spawn,
             mm_ctx::{MmArc, MmCtx, MmCtxBuilder},
             privkey::key_pair_from_seed};
use futures::{channel::mpsc, lock::Mutex as AsyncMutex, StreamExt};
use mm2_libp2p::atomicdex_behaviour::AdexBehaviourCmd;
use mm2_libp2p::{decode_message, PeerId};
use mocktopus::mocking::*;
use rand::{seq::SliceRandom, thread_rng, Rng};
use std::collections::HashSet;
use std::iter::{self, FromIterator};
use std::sync::Mutex;

#[test]
fn test_match_maker_order_and_taker_request() {
    let maker = MakerOrder {
        base: "BASE".into(),
        rel: "REL".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        max_base_vol: 10.into(),
        min_base_vol: 0.into(),
        price: 1.into(),
        matches: HashMap::new(),
        started_swaps: Vec::new(),
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 10.into(),
        rel_amount: 20.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let actual = maker.match_with_request(&request);
    let expected = OrderMatchResult::Matched((10.into(), 10.into()));
    assert_eq!(expected, actual);

    let maker = MakerOrder {
        base: "BASE".into(),
        rel: "REL".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        max_base_vol: 10.into(),
        min_base_vol: 0.into(),
        price: "0.5".into(),
        matches: HashMap::new(),
        started_swaps: Vec::new(),
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 10.into(),
        rel_amount: 20.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let actual = maker.match_with_request(&request);
    let expected = OrderMatchResult::Matched((10.into(), 5.into()));
    assert_eq!(expected, actual);

    let maker = MakerOrder {
        base: "BASE".into(),
        rel: "REL".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        max_base_vol: 10.into(),
        min_base_vol: 0.into(),
        price: "0.5".into(),
        matches: HashMap::new(),
        started_swaps: Vec::new(),
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 10.into(),
        rel_amount: 2.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let actual = maker.match_with_request(&request);
    let expected = OrderMatchResult::NotMatched;
    assert_eq!(expected, actual);

    let maker = MakerOrder {
        base: "BASE".into(),
        rel: "REL".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        max_base_vol: 10.into(),
        min_base_vol: 0.into(),
        price: "0.5".into(),
        matches: HashMap::new(),
        started_swaps: Vec::new(),
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };

    let request = TakerRequest {
        base: "REL".into(),
        rel: "BASE".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 5.into(),
        rel_amount: 10.into(),
        action: TakerAction::Sell,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let actual = maker.match_with_request(&request);
    let expected = OrderMatchResult::Matched((10.into(), 5.into()));
    assert_eq!(expected, actual);

    let maker = MakerOrder {
        base: "BASE".into(),
        rel: "REL".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        max_base_vol: 20.into(),
        min_base_vol: 0.into(),
        price: "0.5".into(),
        matches: HashMap::new(),
        started_swaps: Vec::new(),
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };

    let request = TakerRequest {
        base: "REL".into(),
        rel: "BASE".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 10.into(),
        rel_amount: 10.into(),
        action: TakerAction::Sell,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let actual = maker.match_with_request(&request);
    let expected = OrderMatchResult::Matched((20.into(), 10.into()));
    assert_eq!(expected, actual);

    let maker = MakerOrder {
        base: "BASE".into(),
        rel: "REL".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        max_base_vol: 1.into(),
        min_base_vol: 0.into(),
        price: "1".into(),
        matches: HashMap::new(),
        started_swaps: Vec::new(),
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };

    let request = TakerRequest {
        base: "REL".into(),
        rel: "BASE".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: "0.9".into(),
        action: TakerAction::Sell,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let actual = maker.match_with_request(&request);
    let expected = OrderMatchResult::Matched((1.into(), 1.into()));
    assert_eq!(expected, actual);

    // The following Taker request has not to be matched since the resulted base amount it greater than `max_base_vol`.
    // https://github.com/KomodoPlatform/atomicDEX-API/issues/1041#issuecomment-901863864
    let maker = MakerOrder {
        max_base_vol: "0.2928826881884105".into(),
        min_base_vol: 0.into(),
        price: "2643.01935664".into(),
        created_at: now_ms(),
        updated_at: None,
        base: "ETH-BEP20".to_owned(),
        rel: "KMD".to_owned(),
        matches: HashMap::new(),
        started_swaps: vec![],
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };
    let request = TakerRequest {
        base: "KMD".to_owned(),
        rel: "ETH-BEP20".to_owned(),
        base_amount: "774.205645538427044180416545".into(),
        rel_amount: "0.2928826881884105".into(),
        action: TakerAction::Sell,
        uuid: Uuid::new_v4(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };
    let actual = maker.match_with_request(&request);
    assert_eq!(actual, OrderMatchResult::NotMatched);

    // Though the Taker's rel amount is less than the Makers' min base volume '2',
    // the Maker's price is chosen to calculate the result amounts, so we have:
    // `base_amount = taker_base_amount/maker_price = 30/10 = 3`
    // `rel_amount = taker_base_amount = 30`.
    // The order should be matched.
    let maker = MakerOrder {
        max_base_vol: "3".into(),
        min_base_vol: "2".into(),
        price: 10.into(),
        created_at: now_ms(),
        updated_at: None,
        base: "BASE".to_owned(),
        rel: "REL".to_owned(),
        matches: HashMap::new(),
        started_swaps: vec![],
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };
    let request = TakerRequest {
        base: "REL".to_owned(),
        rel: "BASE".to_owned(),
        base_amount: "30".into(),
        rel_amount: "1.5".into(),
        action: TakerAction::Sell,
        uuid: Uuid::new_v4(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };
    let actual = maker.match_with_request(&request);
    let expected_base_amount = MmNumber::from(3);
    let expected_rel_amount = MmNumber::from(30);
    let expected = OrderMatchResult::Matched((expected_base_amount, expected_rel_amount));
    assert_eq!(actual, expected);
}

// https://github.com/KomodoPlatform/atomicDEX-API/pull/739#discussion_r517275495
#[test]
fn maker_order_match_with_request_zero_volumes() {
    let coin = MmCoinEnum::Test(TestCoin::default());

    let maker_order = MakerOrderBuilder::new(&coin, &coin)
        .with_max_base_vol(1.into())
        .with_price(1.into())
        .build_unchecked();

    // default taker order has empty coins and zero amounts so it should pass to the price calculation stage (division)
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_rel_amount(1.into())
        .build_unchecked();

    let expected = OrderMatchResult::NotMatched;
    let actual = maker_order.match_with_request(&taker_order.request);
    assert_eq!(expected, actual);

    // default taker order has empty coins and zero amounts so it should pass to the price calculation stage (division)
    let taker_request = TakerOrderBuilder::new(&coin, &coin)
        .with_base_amount(1.into())
        .with_action(TakerAction::Sell)
        .build_unchecked();

    let expected = OrderMatchResult::NotMatched;
    let actual = maker_order.match_with_request(&taker_request.request);
    assert_eq!(expected, actual);
}

#[test]
fn test_maker_order_available_amount() {
    let mut maker = MakerOrder {
        base: "BASE".into(),
        rel: "REL".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        max_base_vol: 10.into(),
        min_base_vol: 0.into(),
        price: 1.into(),
        matches: HashMap::new(),
        started_swaps: Vec::new(),
        uuid: Uuid::new_v4(),
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    };
    maker.matches.insert(Uuid::new_v4(), MakerMatch {
        request: TakerRequest {
            uuid: Uuid::new_v4(),
            base: "BASE".into(),
            rel: "REL".into(),
            base_amount: 5.into(),
            rel_amount: 5.into(),
            sender_pubkey: H256Json::default(),
            dest_pub_key: H256Json::default(),
            action: TakerAction::Buy,
            match_by: MatchBy::Any,
            conf_settings: None,
            base_protocol_info: None,
            rel_protocol_info: None,
        },
        reserved: MakerReserved {
            base: "BASE".into(),
            rel: "REL".into(),
            base_amount: 5.into(),
            rel_amount: 5.into(),
            sender_pubkey: H256Json::default(),
            dest_pub_key: H256Json::default(),
            maker_order_uuid: Uuid::new_v4(),
            taker_order_uuid: Uuid::new_v4(),
            conf_settings: None,
            base_protocol_info: None,
            rel_protocol_info: None,
        },
        connect: None,
        connected: None,
        last_updated: now_ms(),
    });
    maker.matches.insert(Uuid::new_v4(), MakerMatch {
        request: TakerRequest {
            uuid: Uuid::new_v4(),
            base: "BASE".into(),
            rel: "REL".into(),
            base_amount: 1.into(),
            rel_amount: 1.into(),
            sender_pubkey: H256Json::default(),
            dest_pub_key: H256Json::default(),
            action: TakerAction::Buy,
            match_by: MatchBy::Any,
            conf_settings: None,
            base_protocol_info: None,
            rel_protocol_info: None,
        },
        reserved: MakerReserved {
            base: "BASE".into(),
            rel: "REL".into(),
            base_amount: 1.into(),
            rel_amount: 1.into(),
            sender_pubkey: H256Json::default(),
            dest_pub_key: H256Json::default(),
            maker_order_uuid: Uuid::new_v4(),
            taker_order_uuid: Uuid::new_v4(),
            conf_settings: None,
            base_protocol_info: None,
            rel_protocol_info: None,
        },
        connect: None,
        connected: None,
        last_updated: now_ms(),
    });

    let expected = BigRational::from_integer(4.into());
    let actual = maker.available_amount();
    assert_eq!(MmNumber::from(expected), actual);
}

#[test]
fn test_taker_match_reserved() {
    let uuid = Uuid::new_v4();

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 10.into(),
        rel_amount: 10.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "BASE".into(),
        rel: "REL".into(),
        base_amount: 10.into(),
        rel_amount: 10.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 10.into(),
        rel_amount: 10.into(),
        action: TakerAction::Sell,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "REL".into(),
        rel: "BASE".into(),
        base_amount: 10.into(),
        rel_amount: 10.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: "0.9".into(),
        action: TakerAction::Sell,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "REL".into(),
        rel: "BASE".into(),
        base_amount: 1.into(),
        rel_amount: 1.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: "0.9".into(),
        action: TakerAction::Sell,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "REL".into(),
        rel: "BASE".into(),
        base_amount: "0.8".into(),
        rel_amount: 1.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::NotMatched, order.match_reserved(&reserved));

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: 2.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "BASE".into(),
        rel: "REL".into(),
        base_amount: 1.into(),
        rel_amount: 1.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: 2.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "BASE".into(),
        rel: "REL".into(),
        base_amount: 1.into(),
        rel_amount: 1.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: 2.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "BASE".into(),
        rel: "REL".into(),
        base_amount: 1.into(),
        rel_amount: 1.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: 2.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "BASE".into(),
        rel: "REL".into(),
        base_amount: 1.into(),
        rel_amount: 3.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::NotMatched, order.match_reserved(&reserved));

    let order = TakerOrder {
        created_at: 1568358064115,
        request: TakerRequest {
            base: "RICK".into(),
            rel: "MORTY".into(),
            base_amount:
                "0.3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333"
                    .into(),
            rel_amount: 1.into(),
            action: TakerAction::Buy,
            uuid,
            sender_pubkey: H256Json::default(),
            dest_pub_key: H256Json::default(),
            match_by: MatchBy::Any,
            conf_settings: None,
            base_protocol_info: None,
            rel_protocol_info: None,
        },
        matches: HashMap::new(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "RICK".into(),
        rel: "MORTY".into(),
        base_amount: "0.3333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333333".into(),
        rel_amount: "0.777777776666666666666666666666666666666666666666666666666666666666666666666666666666666666666666666588888889".into(),
        taker_order_uuid: uuid,
        maker_order_uuid: uuid,
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));
}

#[test]
fn test_taker_order_cancellable() {
    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: 2.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    assert!(order.is_cancellable());

    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid: Uuid::new_v4(),
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 1.into(),
        rel_amount: 2.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Any,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let mut order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    order.matches.insert(Uuid::new_v4(), TakerMatch {
        last_updated: now_ms(),
        reserved: MakerReserved {
            base: "BASE".into(),
            rel: "REL".into(),
            base_amount: 1.into(),
            rel_amount: 3.into(),
            sender_pubkey: H256Json::default(),
            dest_pub_key: H256Json::default(),
            maker_order_uuid: Uuid::new_v4(),
            taker_order_uuid: Uuid::new_v4(),
            conf_settings: None,
            base_protocol_info: None,
            rel_protocol_info: None,
        },
        connect: TakerConnect {
            sender_pubkey: H256Json::default(),
            dest_pub_key: H256Json::default(),
            maker_order_uuid: Uuid::new_v4(),
            taker_order_uuid: Uuid::new_v4(),
        },
        connected: None,
    });

    assert!(!order.is_cancellable());
}

fn prepare_for_cancel_by(ctx: &MmArc) -> mpsc::Receiver<AdexBehaviourCmd> {
    let (tx, rx) = mpsc::channel(10);
    let p2p_ctx = P2PContext::new(tx);
    p2p_ctx.store_to_mm_arc(ctx);

    let ordermatch_ctx = OrdermatchContext::from_ctx(ctx).unwrap();
    let mut maker_orders = block_on(ordermatch_ctx.my_maker_orders.lock());
    let mut taker_orders = block_on(ordermatch_ctx.my_taker_orders.lock());

    maker_orders.insert(Uuid::from_bytes([0; 16]), MakerOrder {
        uuid: Uuid::from_bytes([0; 16]),
        base: "RICK".into(),
        rel: "MORTY".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        matches: HashMap::new(),
        max_base_vol: 0.into(),
        min_base_vol: 0.into(),
        price: 0.into(),
        started_swaps: vec![],
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    });
    maker_orders.insert(Uuid::from_bytes([1; 16]), MakerOrder {
        uuid: Uuid::from_bytes([1; 16]),
        base: "MORTY".into(),
        rel: "RICK".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        matches: HashMap::new(),
        max_base_vol: 0.into(),
        min_base_vol: 0.into(),
        price: 0.into(),
        started_swaps: vec![],
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    });
    maker_orders.insert(Uuid::from_bytes([2; 16]), MakerOrder {
        uuid: Uuid::from_bytes([2; 16]),
        base: "MORTY".into(),
        rel: "ETH".into(),
        created_at: now_ms(),
        updated_at: Some(now_ms()),
        matches: HashMap::new(),
        max_base_vol: 0.into(),
        min_base_vol: 0.into(),
        price: 0.into(),
        started_swaps: vec![],
        conf_settings: None,
        changes_history: None,
        save_in_history: false,
    });
    taker_orders.insert(Uuid::from_bytes([3; 16]), TakerOrder {
        matches: HashMap::new(),
        created_at: now_ms(),
        request: TakerRequest {
            base: "RICK".into(),
            rel: "MORTY".into(),
            uuid: Uuid::from_bytes([3; 16]),
            action: TakerAction::Buy,
            base_amount: 0.into(),
            rel_amount: 0.into(),
            dest_pub_key: H256Json::default(),
            sender_pubkey: H256Json::default(),
            match_by: MatchBy::Any,
            conf_settings: None,
            base_protocol_info: None,
            rel_protocol_info: None,
        },
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    });
    rx
}

#[test]
fn test_cancel_by_single_coin() {
    let ctx = MmCtxBuilder::default()
        .with_secp256k1_key_pair(key_pair_from_seed("123").unwrap())
        .into_mm_arc();
    let rx = prepare_for_cancel_by(&ctx);

    let connection = Connection::open_in_memory().unwrap();
    let _ = ctx.sqlite_connection.pin(Mutex::new(connection));

    delete_my_maker_order.mock_safe(|_, _, _| MockResult::Return(()));
    delete_my_taker_order.mock_safe(|_, _, _| MockResult::Return(()));

    let (cancelled, _) = block_on(cancel_orders_by(&ctx, CancelBy::Coin { ticker: "RICK".into() })).unwrap();
    block_on(rx.take(2).collect::<Vec<_>>());
    assert!(cancelled.contains(&Uuid::from_bytes([0; 16])));
    assert!(cancelled.contains(&Uuid::from_bytes([1; 16])));
    assert!(!cancelled.contains(&Uuid::from_bytes([2; 16])));
    assert!(cancelled.contains(&Uuid::from_bytes([3; 16])));
}

#[test]
fn test_cancel_by_pair() {
    let ctx = MmCtxBuilder::default()
        .with_secp256k1_key_pair(key_pair_from_seed("123").unwrap())
        .into_mm_arc();
    let rx = prepare_for_cancel_by(&ctx);

    let connection = Connection::open_in_memory().unwrap();
    let _ = ctx.sqlite_connection.pin(Mutex::new(connection));

    delete_my_maker_order.mock_safe(|_, _, _| MockResult::Return(()));
    delete_my_taker_order.mock_safe(|_, _, _| MockResult::Return(()));

    let (cancelled, _) = block_on(cancel_orders_by(&ctx, CancelBy::Pair {
        base: "RICK".into(),
        rel: "MORTY".into(),
    }))
    .unwrap();
    block_on(rx.take(1).collect::<Vec<_>>());
    assert!(cancelled.contains(&Uuid::from_bytes([0; 16])));
    assert!(!cancelled.contains(&Uuid::from_bytes([1; 16])));
    assert!(!cancelled.contains(&Uuid::from_bytes([2; 16])));
    assert!(cancelled.contains(&Uuid::from_bytes([3; 16])));
}

#[test]
fn test_cancel_by_all() {
    let ctx = MmCtxBuilder::default()
        .with_secp256k1_key_pair(key_pair_from_seed("123").unwrap())
        .into_mm_arc();
    let rx = prepare_for_cancel_by(&ctx);

    let connection = Connection::open_in_memory().unwrap();
    let _ = ctx.sqlite_connection.pin(Mutex::new(connection));

    delete_my_maker_order.mock_safe(|_, _, _| MockResult::Return(()));
    delete_my_taker_order.mock_safe(|_, _, _| MockResult::Return(()));

    let (cancelled, _) = block_on(cancel_orders_by(&ctx, CancelBy::All)).unwrap();
    block_on(rx.take(3).collect::<Vec<_>>());
    assert!(cancelled.contains(&Uuid::from_bytes([0; 16])));
    assert!(cancelled.contains(&Uuid::from_bytes([1; 16])));
    assert!(cancelled.contains(&Uuid::from_bytes([2; 16])));
    assert!(cancelled.contains(&Uuid::from_bytes([3; 16])));
}

#[test]
// https://github.com/KomodoPlatform/atomicDEX-API/issues/607
fn test_taker_order_match_by() {
    let uuid = Uuid::new_v4();

    let mut not_matching_uuids = HashSet::new();
    not_matching_uuids.insert(Uuid::new_v4());
    let request = TakerRequest {
        base: "BASE".into(),
        rel: "REL".into(),
        uuid,
        dest_pub_key: H256Json::default(),
        sender_pubkey: H256Json::default(),
        base_amount: 10.into(),
        rel_amount: 10.into(),
        action: TakerAction::Buy,
        match_by: MatchBy::Orders(not_matching_uuids),
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    let mut order = TakerOrder {
        request,
        matches: HashMap::new(),
        created_at: now_ms(),
        order_type: OrderType::GoodTillCancelled,
        min_volume: 0.into(),
        timeout: 30,
        save_in_history: false,
    };

    let reserved = MakerReserved {
        base: "BASE".into(),
        rel: "REL".into(),
        base_amount: 10.into(),
        rel_amount: 10.into(),
        sender_pubkey: H256Json::default(),
        dest_pub_key: H256Json::default(),
        maker_order_uuid: Uuid::new_v4(),
        taker_order_uuid: uuid,
        conf_settings: None,
        base_protocol_info: None,
        rel_protocol_info: None,
    };

    assert_eq!(MatchReservedResult::NotMatched, order.match_reserved(&reserved));

    let mut matching_uuids = HashSet::new();
    matching_uuids.insert(reserved.maker_order_uuid);
    order.request.match_by = MatchBy::Orders(matching_uuids);
    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));

    let mut not_matching_pubkeys = HashSet::new();
    not_matching_pubkeys.insert([1; 32].into());
    order.request.match_by = MatchBy::Pubkeys(not_matching_pubkeys);
    assert_eq!(MatchReservedResult::NotMatched, order.match_reserved(&reserved));

    let mut matching_pubkeys = HashSet::new();
    matching_pubkeys.insert(H256Json::default());
    order.request.match_by = MatchBy::Pubkeys(matching_pubkeys);
    assert_eq!(MatchReservedResult::Matched, order.match_reserved(&reserved));
}

#[test]
fn lp_connect_start_bob_should_not_be_invoked_if_order_match_already_connected() {
    let order_json = r#"{"max_base_vol":"1","max_base_vol_rat":[[1,[1]],[1,[1]]],"min_base_vol":"0","min_base_vol_rat":[[0,[]],[1,[1]]],"price":"1","price_rat":[[1,[1]],[1,[1]]],"created_at":1589265312093,"updated_at":1589265312093,"base":"ETH","rel":"JST","matches":{"2f9afe84-7a89-4194-8947-45fba563118f":{"request":{"base":"ETH","rel":"JST","base_amount":"0.1","base_amount_rat":[[1,[1]],[1,[10]]],"rel_amount":"0.2","rel_amount_rat":[[1,[1]],[1,[5]]],"action":"Buy","uuid":"2f9afe84-7a89-4194-8947-45fba563118f","method":"request","sender_pubkey":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3","dest_pub_key":"0000000000000000000000000000000000000000000000000000000000000000","match_by":{"type":"Any"}},"reserved":{"base":"ETH","rel":"JST","base_amount":"0.1","base_amount_rat":[[1,[1]],[1,[10]]],"rel_amount":"0.1","rel_amount_rat":[[1,[1]],[1,[10]]],"taker_order_uuid":"2f9afe84-7a89-4194-8947-45fba563118f","maker_order_uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3","method":"reserved","sender_pubkey":"c6a78589e18b482aea046975e6d0acbdea7bf7dbf04d9d5bd67fda917815e3ed","dest_pub_key":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3"},"connect":{"taker_order_uuid":"2f9afe84-7a89-4194-8947-45fba563118f","maker_order_uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3","method":"connect","sender_pubkey":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3","dest_pub_key":"c6a78589e18b482aea046975e6d0acbdea7bf7dbf04d9d5bd67fda917815e3ed"},"connected":{"taker_order_uuid":"2f9afe84-7a89-4194-8947-45fba563118f","maker_order_uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3","method":"connected","sender_pubkey":"c6a78589e18b482aea046975e6d0acbdea7bf7dbf04d9d5bd67fda917815e3ed","dest_pub_key":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3"},"last_updated":1589265314408}},"started_swaps":["2f9afe84-7a89-4194-8947-45fba563118f"],"uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3"}"#;
    let maker_order: MakerOrder = json::from_str(order_json).unwrap();
    let ctx = MmCtxBuilder::default()
        .with_secp256k1_key_pair(
            key_pair_from_seed("also shoot benefit prefer juice shell elder veteran woman mimic image kidney").unwrap(),
        )
        .into_mm_arc();
    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    block_on(ordermatch_ctx.my_maker_orders.lock()).insert(maker_order.uuid, maker_order);

    static mut CONNECT_START_CALLED: bool = false;
    lp_connect_start_bob.mock_safe(|_, _, _| {
        MockResult::Return(unsafe {
            CONNECT_START_CALLED = true;
        })
    });

    let connect: TakerConnect = json::from_str(r#"{"taker_order_uuid":"2f9afe84-7a89-4194-8947-45fba563118f","maker_order_uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3","method":"connect","sender_pubkey":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3","dest_pub_key":"c6a78589e18b482aea046975e6d0acbdea7bf7dbf04d9d5bd67fda917815e3ed"}"#).unwrap();
    block_on(process_taker_connect(ctx, connect.sender_pubkey.clone(), connect));
    assert!(unsafe { !CONNECT_START_CALLED });
}

#[test]
fn should_process_request_only_once() {
    let order_json = r#"{"max_base_vol":"1","max_base_vol_rat":[[1,[1]],[1,[1]]],"min_base_vol":"0","min_base_vol_rat":[[0,[]],[1,[1]]],"price":"1","price_rat":[[1,[1]],[1,[1]]],"created_at":1589265312093,"updated_at":1589265312093,"base":"ETH","rel":"JST","matches":{"2f9afe84-7a89-4194-8947-45fba563118f":{"request":{"base":"ETH","rel":"JST","base_amount":"0.1","base_amount_rat":[[1,[1]],[1,[10]]],"rel_amount":"0.2","rel_amount_rat":[[1,[1]],[1,[5]]],"action":"Buy","uuid":"2f9afe84-7a89-4194-8947-45fba563118f","method":"request","sender_pubkey":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3","dest_pub_key":"0000000000000000000000000000000000000000000000000000000000000000","match_by":{"type":"Any"}},"reserved":{"base":"ETH","rel":"JST","base_amount":"0.1","base_amount_rat":[[1,[1]],[1,[10]]],"rel_amount":"0.1","rel_amount_rat":[[1,[1]],[1,[10]]],"taker_order_uuid":"2f9afe84-7a89-4194-8947-45fba563118f","maker_order_uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3","method":"reserved","sender_pubkey":"c6a78589e18b482aea046975e6d0acbdea7bf7dbf04d9d5bd67fda917815e3ed","dest_pub_key":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3"},"connect":{"taker_order_uuid":"2f9afe84-7a89-4194-8947-45fba563118f","maker_order_uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3","method":"connect","sender_pubkey":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3","dest_pub_key":"c6a78589e18b482aea046975e6d0acbdea7bf7dbf04d9d5bd67fda917815e3ed"},"connected":{"taker_order_uuid":"2f9afe84-7a89-4194-8947-45fba563118f","maker_order_uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3","method":"connected","sender_pubkey":"c6a78589e18b482aea046975e6d0acbdea7bf7dbf04d9d5bd67fda917815e3ed","dest_pub_key":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3"},"last_updated":1589265314408}},"started_swaps":["2f9afe84-7a89-4194-8947-45fba563118f"],"uuid":"5f6516ea-ccaa-453a-9e37-e1c2c0d527e3"}"#;
    let maker_order: MakerOrder = json::from_str(order_json).unwrap();
    let uuid = maker_order.uuid;
    let ctx = MmCtxBuilder::default()
        .with_secp256k1_key_pair(
            key_pair_from_seed("also shoot benefit prefer juice shell elder veteran woman mimic image kidney").unwrap(),
        )
        .into_mm_arc();
    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    block_on(ordermatch_ctx.my_maker_orders.lock()).insert(maker_order.uuid, maker_order);
    let request: TakerRequest = json::from_str(
        r#"{"base":"ETH","rel":"JST","base_amount":"0.1","base_amount_rat":[[1,[1]],[1,[10]]],"rel_amount":"0.2","rel_amount_rat":[[1,[1]],[1,[5]]],"action":"Buy","uuid":"2f9afe84-7a89-4194-8947-45fba563118f","method":"request","sender_pubkey":"031d4256c4bc9f99ac88bf3dba21773132281f65f9bf23a59928bce08961e2f3","dest_pub_key":"0000000000000000000000000000000000000000000000000000000000000000","match_by":{"type":"Any"}}"#,
    ).unwrap();
    block_on(process_taker_request(ctx, Default::default(), request));
    let maker_orders = block_on(ordermatch_ctx.my_maker_orders.lock());
    let order = maker_orders.get(&uuid).unwrap();
    // when new request is processed match is replaced with new instance resetting
    // connect and connected to None so by checking is_some we check that request message is ignored
    assert!(order
        .matches
        .get(&"2f9afe84-7a89-4194-8947-45fba563118f".parse().unwrap())
        .unwrap()
        .connect
        .is_some());
    assert!(order
        .matches
        .get(&"2f9afe84-7a89-4194-8947-45fba563118f".parse().unwrap())
        .unwrap()
        .connected
        .is_some());
}

#[test]
fn test_choose_maker_confs_settings() {
    let coin = TestCoin::default().into();
    // no confs set
    let taker_order = TakerOrderBuilder::new(&coin, &coin).build_unchecked();
    TestCoin::requires_notarization.mock_safe(|_| MockResult::Return(true));
    TestCoin::required_confirmations.mock_safe(|_| MockResult::Return(8));
    let settings = choose_maker_confs_and_notas(None, &taker_order.request, &coin, &coin);
    // should pick settings from coin configuration
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 8);
    assert!(settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 8);

    let maker_conf_settings = OrderConfirmationsSettings {
        base_confs: 1,
        base_nota: false,
        rel_confs: 1,
        rel_nota: false,
    };
    // no confs set
    let taker_order = TakerOrderBuilder::new(&coin, &coin).build_unchecked();
    let settings = choose_maker_confs_and_notas(Some(maker_conf_settings), &taker_order.request, &coin, &coin);
    // should pick settings from maker order
    assert!(!settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 1);
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);

    let maker_conf_settings = OrderConfirmationsSettings {
        base_confs: 10,
        base_nota: true,
        rel_confs: 1,
        rel_nota: false,
    };
    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 5,
        base_nota: false,
        rel_confs: 5,
        rel_nota: false,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let settings = choose_maker_confs_and_notas(Some(maker_conf_settings), &taker_order.request, &coin, &coin);
    // should pick settings from taker request because taker will wait less time for our
    // payment confirmation
    assert!(!settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 5);
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);

    let maker_conf_settings = OrderConfirmationsSettings {
        base_confs: 10,
        base_nota: false,
        rel_confs: 1,
        rel_nota: false,
    };
    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 1000,
        base_nota: true,
        rel_confs: 1000,
        rel_nota: true,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let settings = choose_maker_confs_and_notas(Some(maker_conf_settings), &taker_order.request, &coin, &coin);
    // keep using our settings allowing taker to wait for our payment conf as much as he likes
    assert!(!settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 10);
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);

    let maker_conf_settings = OrderConfirmationsSettings {
        base_confs: 10,
        base_nota: false,
        rel_confs: 2,
        rel_nota: true,
    };

    let taker_conf_settings = OrderConfirmationsSettings {
        rel_confs: 1,
        rel_nota: false,
        base_confs: 1,
        base_nota: false,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let settings = choose_maker_confs_and_notas(Some(maker_conf_settings), &taker_order.request, &coin, &coin);

    // Taker conf settings should not have any effect on maker conf requirements for taker payment
    assert!(settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 2);

    let maker_conf_settings = OrderConfirmationsSettings {
        base_confs: 10,
        base_nota: true,
        rel_confs: 1,
        rel_nota: false,
    };
    // Pair is reversed for TakerAction::Sell
    let taker_conf_settings = OrderConfirmationsSettings {
        rel_confs: 5,
        rel_nota: false,
        base_confs: 5,
        base_nota: false,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .with_action(TakerAction::Sell)
        .build_unchecked();
    let settings = choose_maker_confs_and_notas(Some(maker_conf_settings), &taker_order.request, &coin, &coin);
    // should pick settings from taker request because taker will wait less time for our
    // payment confirmation
    assert!(!settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 5);
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);
}

#[test]
fn test_choose_taker_confs_settings_buy_action() {
    let coin = TestCoin::default().into();

    // no confs and notas set
    let taker_order = TakerOrderBuilder::new(&coin, &coin).build_unchecked();
    // no confs and notas set
    let maker_reserved = MakerReserved::default();
    TestCoin::requires_notarization.mock_safe(|_| MockResult::Return(true));
    TestCoin::required_confirmations.mock_safe(|_| MockResult::Return(8));
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should pick settings from coins
    assert!(settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 8);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 8);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 5,
        base_nota: true,
        rel_confs: 4,
        rel_nota: false,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    // no confs and notas set
    let maker_reserved = MakerReserved::default();
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should pick settings from taker request
    // as action is buy my_coin is rel and other coin is base
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 4);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 5);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 2,
        base_nota: true,
        rel_confs: 2,
        rel_nota: true,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let mut maker_reserved = MakerReserved::default();
    let maker_conf_settings = OrderConfirmationsSettings {
        rel_confs: 1,
        rel_nota: false,
        base_confs: 2,
        base_nota: true,
    };
    maker_reserved.conf_settings = Some(maker_conf_settings);
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should pick settings from maker reserved if he requires less confs
    // as action is buy my_coin is rel and other coin is base in request
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 2);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 2,
        base_nota: true,
        rel_confs: 1,
        rel_nota: false,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let mut maker_reserved = MakerReserved::default();
    let maker_conf_settings = OrderConfirmationsSettings {
        rel_confs: 2,
        rel_nota: true,
        base_confs: 2,
        base_nota: true,
    };
    maker_reserved.conf_settings = Some(maker_conf_settings);
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should allow maker to use more confirmations than we require, but it shouldn't affect our settings
    // as action is buy my_coin is rel and other coin is base in request
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 2);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 2,
        base_nota: true,
        rel_confs: 1,
        rel_nota: false,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let mut maker_reserved = MakerReserved::default();
    let maker_conf_settings = OrderConfirmationsSettings {
        base_confs: 1,
        base_nota: false,
        rel_confs: 2,
        rel_nota: true,
    };
    maker_reserved.conf_settings = Some(maker_conf_settings);
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // maker settings should have no effect on other_coin_confs and other_coin_nota
    // as action is buy my_coin is rel and other coin is base in request
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 2);
}

#[test]
fn test_choose_taker_confs_settings_sell_action() {
    let coin = TestCoin::default().into();

    // no confs and notas set
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_action(TakerAction::Sell)
        .build_unchecked();
    // no confs and notas set
    let maker_reserved = MakerReserved::default();
    TestCoin::requires_notarization.mock_safe(|_| MockResult::Return(true));
    TestCoin::required_confirmations.mock_safe(|_| MockResult::Return(8));
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should pick settings from coins
    assert!(settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 8);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 8);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 4,
        base_nota: false,
        rel_confs: 5,
        rel_nota: true,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_action(TakerAction::Sell)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    // no confs and notas set
    let maker_reserved = MakerReserved::default();
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should pick settings from taker request
    // as action is sell my_coin is base and other coin is rel in request
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 4);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 5);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 2,
        base_nota: true,
        rel_confs: 2,
        rel_nota: true,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_action(TakerAction::Sell)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let mut maker_reserved = MakerReserved::default();
    let maker_conf_settings = OrderConfirmationsSettings {
        base_confs: 2,
        base_nota: true,
        rel_confs: 1,
        rel_nota: false,
    };
    maker_reserved.conf_settings = Some(maker_conf_settings);
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should pick settings from maker reserved if he requires less confs
    // as action is sell my_coin is base and other coin is rel in request
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 2);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 1,
        base_nota: false,
        rel_confs: 2,
        rel_nota: true,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_action(TakerAction::Sell)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let mut maker_reserved = MakerReserved::default();
    let maker_conf_settings = OrderConfirmationsSettings {
        rel_confs: 2,
        rel_nota: true,
        base_confs: 1,
        base_nota: false,
    };
    maker_reserved.conf_settings = Some(maker_conf_settings);
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // should allow maker to use more confirmations than we require, but it shouldn't affect our settings
    // as action is sell my_coin is base and other coin is rel in request
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 2);

    let taker_conf_settings = OrderConfirmationsSettings {
        base_confs: 1,
        base_nota: false,
        rel_confs: 2,
        rel_nota: true,
    };
    let taker_order = TakerOrderBuilder::new(&coin, &coin)
        .with_action(TakerAction::Sell)
        .with_conf_settings(taker_conf_settings)
        .build_unchecked();
    let mut maker_reserved = MakerReserved::default();
    let maker_conf_settings = OrderConfirmationsSettings {
        rel_confs: 2,
        rel_nota: true,
        base_confs: 1,
        base_nota: false,
    };
    maker_reserved.conf_settings = Some(maker_conf_settings);
    let settings = choose_taker_confs_and_notas(&taker_order.request, &maker_reserved, &coin, &coin);
    // maker settings should have no effect on other_coin_confs and other_coin_nota
    // as action is sell my_coin is base and other coin is rel in request
    assert!(!settings.taker_coin_nota);
    assert_eq!(settings.taker_coin_confs, 1);
    assert!(settings.maker_coin_nota);
    assert_eq!(settings.maker_coin_confs, 2);
}

fn make_ctx_for_tests() -> (MmArc, String, [u8; 32]) {
    let ctx = MmArc(Arc::new(MmCtx::default()));
    ctx.init_metrics().unwrap();
    ctx.secp256k1_key_pair
        .pin(key_pair_from_seed("passphrase").unwrap())
        .unwrap();
    let secret = *(&*ctx.secp256k1_key_pair().private().secret);
    let pubkey = hex::encode(&**ctx.secp256k1_key_pair().public());
    (ctx, pubkey, secret)
}

fn make_random_orders(pubkey: String, _secret: &[u8; 32], base: String, rel: String, n: usize) -> Vec<OrderbookItem> {
    let mut rng = rand::thread_rng();
    let mut orders = Vec::with_capacity(n);
    for _i in 0..n {
        let numer: u64 = rng.gen_range(2000, 10000000);
        let order = new_protocol::MakerOrderCreated {
            uuid: Uuid::new_v4().into(),
            base: base.clone(),
            rel: rel.clone(),
            price: BigRational::new(numer.into(), 1000000.into()),
            max_volume: BigRational::from_integer(1.into()),
            min_volume: BigRational::from_integer(0.into()),
            conf_settings: OrderConfirmationsSettings::default(),
            created_at: now_ms() / 1000,
            timestamp: now_ms() / 1000,
            pair_trie_root: H64::default(),
        };

        orders.push((order, pubkey.clone()).into());
    }

    orders
}

fn pubkey_and_secret_for_test(passphrase: &str) -> (String, [u8; 32]) {
    let key_pair = key_pair_from_seed(passphrase).unwrap();
    let pubkey = hex::encode(&**key_pair.public());
    let secret = *(&*key_pair.private().secret);
    (pubkey, secret)
}

fn p2p_context_mock() -> (mpsc::Sender<AdexBehaviourCmd>, mpsc::Receiver<AdexBehaviourCmd>) {
    let (cmd_tx, cmd_rx) = mpsc::channel(10);
    let cmd_sender = cmd_tx.clone();
    P2PContext::fetch_from_mm_arc.mock_safe(move |_| {
        MockResult::Return(Arc::new(P2PContext {
            cmd_tx: AsyncMutex::new(cmd_sender.clone()),
        }))
    });
    (cmd_tx, cmd_rx)
}

#[test]
fn test_process_get_orderbook_request() {
    const ORDERS_NUMBER: usize = 10;

    let (ctx, _pubkey, _secret) = make_ctx_for_tests();
    let (pubkey1, secret1) = pubkey_and_secret_for_test("passphrase-1");
    let (pubkey2, secret2) = pubkey_and_secret_for_test("passphrase-2");
    let (pubkey3, secret3) = pubkey_and_secret_for_test("passphrase-3");

    let mut pubkey1_orders =
        make_random_orders(pubkey1.clone(), &secret1, "RICK".into(), "MORTY".into(), ORDERS_NUMBER);
    let mut pubkey2_orders =
        make_random_orders(pubkey2.clone(), &secret2, "MORTY".into(), "RICK".into(), ORDERS_NUMBER);
    let mut pubkey3_orders =
        make_random_orders(pubkey3.clone(), &secret3, "RICK".into(), "MORTY".into(), ORDERS_NUMBER);
    pubkey3_orders.extend_from_slice(&make_random_orders(
        pubkey3.clone(),
        &secret3,
        "MORTY".into(),
        "RICK".into(),
        ORDERS_NUMBER,
    ));

    pubkey1_orders.sort_unstable_by(|x, y| x.uuid.cmp(&y.uuid));
    pubkey2_orders.sort_unstable_by(|x, y| x.uuid.cmp(&y.uuid));
    pubkey3_orders.sort_unstable_by(|x, y| x.uuid.cmp(&y.uuid));

    let mut orders_by_pubkeys = HashMap::new();
    orders_by_pubkeys.insert(pubkey1, pubkey1_orders);
    orders_by_pubkeys.insert(pubkey2, pubkey2_orders);
    orders_by_pubkeys.insert(pubkey3, pubkey3_orders);

    let ordermatch_ctx = Arc::new(OrdermatchContext::default());
    let ordermatch_ctx_clone = ordermatch_ctx.clone();
    OrdermatchContext::from_ctx.mock_safe(move |_| MockResult::Return(Ok(ordermatch_ctx_clone.clone())));

    let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());

    for order in orders_by_pubkeys.iter().map(|(_pubkey, orders)| orders).flatten() {
        orderbook.insert_or_update_order_update_trie(order.clone());
    }

    // avoid dead lock on orderbook as process_get_orderbook_request also acquires it
    drop(orderbook);

    let encoded = block_on(process_get_orderbook_request(
        ctx.clone(),
        "RICK".into(),
        "MORTY".into(),
    ))
    .unwrap()
    .unwrap();

    let orderbook = decode_message::<GetOrderbookRes>(&encoded).unwrap();
    for (pubkey, item) in orderbook.pubkey_orders {
        let expected = orders_by_pubkeys
            .get(&pubkey)
            .expect(&format!("!best_orders_by_pubkeys is expected to contain {:?}", pubkey));

        let mut actual: Vec<OrderbookItem> = item.orders.iter().map(|(_uuid, order)| order.clone()).collect();
        actual.sort_unstable_by(|x, y| x.uuid.cmp(&y.uuid));
        log!([pubkey]"-"[actual.len()]);
        assert_eq!(actual, *expected);
    }
}

#[test]
fn test_process_get_orderbook_request_limit() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();

    let ordermatch_ctx = Arc::new(OrdermatchContext::default());
    let ordermatch_ctx_clone = ordermatch_ctx.clone();
    OrdermatchContext::from_ctx.mock_safe(move |_| MockResult::Return(Ok(ordermatch_ctx_clone.clone())));

    let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());

    let orders = make_random_orders(
        pubkey,
        &secret,
        "RICK".into(),
        "MORTY".into(),
        MAX_ORDERS_NUMBER_IN_ORDERBOOK_RESPONSE + 1,
    );

    for order in orders {
        orderbook.insert_or_update_order_update_trie(order);
    }

    // avoid dead lock on orderbook as process_get_orderbook_request also acquires it
    drop(orderbook);

    let err = block_on(process_get_orderbook_request(
        ctx.clone(),
        "RICK".into(),
        "MORTY".into(),
    ))
    .err()
    .expect("Expected an error");

    log!("error: "(err));
    assert!(err.contains("Orderbook too large"));
}

#[test]
fn test_request_and_fill_orderbook() {
    const PUBKEYS_NUMBER: usize = 3;
    const ORDERS_NUMBER: usize = 10;

    let (ctx, _pubkey, _secret) = make_ctx_for_tests();
    let (_, mut cmd_rx) = p2p_context_mock();

    let other_pubkeys: Vec<(String, [u8; 32])> = (0..PUBKEYS_NUMBER)
        .map(|idx| {
            let passphrase = format!("passphrase-{}", idx);
            pubkey_and_secret_for_test(&passphrase)
        })
        .collect();
    let expected_orders: HashMap<String, Vec<(Uuid, OrderbookItem)>> = other_pubkeys
        .iter()
        .map(|(pubkey, secret)| {
            let orders: Vec<_> =
                make_random_orders(pubkey.clone(), secret, "RICK".into(), "MORTY".into(), ORDERS_NUMBER)
                    .into_iter()
                    .map(|order| (order.uuid, order))
                    .collect();
            (pubkey.clone(), orders)
        })
        .collect();

    // insert extra (RICK, MORTY) orders that must be removed from our trie before the orderbook is filled
    {
        let (pubkey, secret) = &other_pubkeys[0];
        for extra_order in make_random_orders(pubkey.clone(), secret, "RICK".into(), "MORTY".into(), 2) {
            block_on(insert_or_update_order(&ctx, extra_order));
        }
    }

    let expected_request = P2PRequest::Ordermatch(OrdermatchRequest::GetOrderbook {
        base: "RICK".into(),
        rel: "MORTY".into(),
    });

    let orders = expected_orders.clone();
    spawn(async move {
        let cmd = cmd_rx.next().await.unwrap();
        let (req, response_tx) = if let AdexBehaviourCmd::RequestAnyRelay { req, response_tx } = cmd {
            (req, response_tx)
        } else {
            panic!("Unexpected cmd");
        };

        // check if the received request is expected
        let actual = decode_message::<P2PRequest>(&req).unwrap();
        assert_eq!(actual, expected_request);

        let result = orders
            .into_iter()
            .map(|(pubkey, orders)| {
                let item = GetOrderbookPubkeyItem {
                    orders,
                    last_keep_alive: now_ms() / 1000,
                    last_signed_pubkey_payload: vec![],
                };
                (pubkey, item)
            })
            .collect();
        let orderbook = GetOrderbookRes { pubkey_orders: result };
        let encoded = encode_message(&orderbook).unwrap();

        // send the response through the response channel
        response_tx.send(Some((PeerId::random(), encoded))).unwrap();
    });

    block_on(request_and_fill_orderbook(&ctx, "RICK", "MORTY")).unwrap();

    // check if the best asks and bids are in the orderbook
    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());

    let expected = expected_orders
        .iter()
        .map(|(_pubkey, orders)| orders.clone())
        .flatten()
        .collect();
    assert_eq!(orderbook.order_set, expected);

    let expected = expected_orders
        .iter()
        .map(|(_pubkey, orders)| orders)
        .flatten()
        .map(|(uuid, _order)| *uuid)
        .collect();
    let unordered = orderbook
        .unordered
        .get(&("RICK".to_owned(), "MORTY".to_owned()))
        .expect("No (RICK, MORTY) in unordered container");
    assert_eq!(*unordered, expected);

    let expected = expected_orders
        .iter()
        .map(|(_pubkey, orders)| orders)
        .flatten()
        .map(|(uuid, order)| OrderedByPriceOrder {
            uuid: *uuid,
            price: order.price.clone().into(),
        })
        .collect();
    let ordered = orderbook
        .ordered
        .get(&("RICK".to_owned(), "MORTY".to_owned()))
        .expect("No (RICK, MORTY) in unordered container");
    assert_eq!(*ordered, expected);

    let rick_morty_pair = alb_ordered_pair("RICK", "MORTY");
    for (pubkey, orders) in expected_orders {
        let pubkey_state = orderbook
            .pubkeys_state
            .get(&pubkey)
            .unwrap_or_else(|| panic!("!pubkey_state.get() {} pubkey", pubkey));

        let expected = orders
            .iter()
            .map(|(uuid, _order)| (*uuid, rick_morty_pair.clone()))
            .collect();
        assert_eq!(pubkey_state.orders_uuids, expected);

        let root = pubkey_state
            .trie_roots
            .get(&rick_morty_pair)
            .unwrap_or_else(|| panic!("!pubkey_state.trie_roots.get() {}", rick_morty_pair));

        // check if the root contains only expected orders
        let trie = TrieDB::<Layout>::new(&orderbook.memory_db, root).expect("!TrieDB::new()");
        let mut in_trie: Vec<(Uuid, OrderbookItem)> = trie
            .iter()
            .expect("!TrieDB::iter()")
            .map(|key_value| {
                let (key, value) = key_value.expect("Iterator returned an error");
                let key = TryFromBytes::try_from_bytes(key).expect("!try_from_bytes() key");
                let value = TryFromBytes::try_from_bytes(value).expect("!try_from_bytes() val");
                (key, value)
            })
            .collect();

        in_trie.sort_by(|x, y| x.0.cmp(&y.0));
        let mut expected = orders;
        expected.sort_by(|x, y| x.0.cmp(&y.0));
        assert_eq!(in_trie, expected);
    }
}

/*
#[test]
fn test_process_order_keep_alive_requested_from_peer() {
    let ordermatch_ctx = Arc::new(OrdermatchContext::default());
    let ordermatch_ctx_clone = ordermatch_ctx.clone();
    OrdermatchContext::from_ctx.mock_safe(move |_| MockResult::Return(Ok(ordermatch_ctx_clone.clone())));
    let (_, mut cmd_rx) = p2p_context_mock();

    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let uuid = Uuid::new_v4();
    let peer = PeerId::random().to_string();

    let order = new_protocol::MakerOrderCreated {
        uuid: uuid.clone().into(),
        base: "RICK".into(),
        rel: "MORTY".into(),
        price: BigRational::from_integer(1000000.into()),
        max_volume: BigRational::from_integer(2000000.into()),
        min_volume: BigRational::from_integer(2000000.into()),
        conf_settings: OrderConfirmationsSettings::default(),
    };

    // create an initial_message and encode it with the secret
    let initial_order_message = encode_and_sign(
        &new_protocol::OrdermatchMessage::MakerOrderCreated(order.clone()),
        &secret,
    )
    .unwrap();

    let expected_request = P2PRequest::Ordermatch(OrdermatchRequest::GetOrders {
        pairs: vec![("RICK".into(), "MORTY".into())],
        from_pubkey: pubkey.clone(),
    });
    let from_peer = peer.clone();
    let initial_message = initial_order_message.clone();
    spawn(async move {
        let cmd = cmd_rx.next().await.unwrap();
        let (req, response_tx) = if let AdexBehaviourCmd::RequestPeers { req, response_tx, .. } = cmd {
            (req, response_tx)
        } else {
            panic!("Unexpected cmd");
        };

        // check if the received request is expected
        let actual = decode_message::<P2PRequest>(&req).unwrap();
        assert_eq!(actual, expected_request);

        // create a response with the initial_message and random from_peer
        let response = vec![new_protocol::OrderInitialMessage {
            initial_message,
            from_peer: from_peer.clone(),
            update_messages: Vec::new(),
        }];

        let response = AdexResponse::Ok {
            response: encode_message(&response).unwrap(),
        };
        response_tx.send(vec![(PeerId::random(), response)]).unwrap();
    });

    let keep_alive = new_protocol::MakerOrdersKeepAlive {
        timestamp: now_ms(),
        num_orders: HashMap::from_iter(iter::once((("RICK".into(), "MORTY".into()), 1))),
    };

    // process_order_keep_alive() should return true because an order was successfully requested from a peer.
    assert!(block_on(process_orders_keep_alive(
        ctx,
        peer.clone(),
        pubkey.clone(),
        keep_alive
    )));

    let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());
    // try to find the order within OrdermatchContext::orderbook and check if this order equals to the expected
    let actual = orderbook.find_order_by_uuid_and_pubkey(&uuid, &pubkey).unwrap();
    let expected: OrderbookItem = (order, pubkey).into();

    assert_eq!(actual, &expected);
}

#[test]
fn test_process_get_order_request() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let ordermatch_ctx = Arc::new(OrdermatchContext::default());
    let ordermatch_ctx_clone = ordermatch_ctx.clone();
    OrdermatchContext::from_ctx.mock_safe(move |_| MockResult::Return(Ok(ordermatch_ctx_clone.clone())));

    let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());

    let order = new_protocol::MakerOrderCreated {
        uuid: Uuid::new_v4().into(),
        base: "RICK".into(),
        rel: "MORTY".into(),
        price: BigRational::from_integer(1000000.into()),
        max_volume: BigRational::from_integer(2000000.into()),
        min_volume: BigRational::from_integer(2000000.into()),
        conf_settings: OrderConfirmationsSettings::default(),
    };
    // create an initial_message and encode it with the secret
    let initial_message = encode_and_sign(
        &new_protocol::OrdermatchMessage::MakerOrderCreated(order.clone()),
        &secret,
    )
    .unwrap();
    let price_ping_request: OrderbookItem = (order, pubkey.clone()).into();
    orderbook.insert_or_update_order(price_ping_request.clone());

    // avoid dead lock on orderbook as process_get_orderbook_request also acquires it
    drop(orderbook);

    let encoded = block_on(process_get_order_request(
        ctx.clone(),
        price_ping_request.uuid,
        pubkey.clone(),
    ))
    .unwrap()
    .unwrap();

    let order = decode_message::<new_protocol::OrderInitialMessage>(&encoded).unwrap();
    let actual_price_ping_request = OrderbookItem::from_initial_msg(order.initial_message, order.from_peer).unwrap();
    assert_eq!(actual_price_ping_request, price_ping_request);
}

#[test]
fn test_subscribe_to_ordermatch_topic_not_subscribed() {
    let (ctx, _pubkey, _secret) = make_ctx_for_tests();
    let (_, mut cmd_rx) = p2p_context_mock();

    spawn(async move {
        match cmd_rx.next().await.unwrap() {
            AdexBehaviourCmd::Subscribe { .. } => (),
            _ => panic!("AdexBehaviourCmd::Subscribe expected first"),
        }

        let (req, response_tx) = match cmd_rx.next().await.unwrap() {
            AdexBehaviourCmd::RequestRelays { req, response_tx } => (req, response_tx),
            _ => panic!("AdexBehaviourCmd::RequestRelays expected"),
        };

        let request = decode_message::<P2PRequest>(&req).unwrap();
        match request {
            P2PRequest::Ordermatch(OrdermatchRequest::GetOrderbook { .. }) => (),
            _ => panic!(),
        }

        let response = new_protocol::Orderbook {
            asks: Vec::new(),
            bids: Vec::new(),
        };
        let encoded = encode_message(&response).unwrap();
        let response = vec![(PeerId::random(), AdexResponse::Ok { response: encoded })];
        response_tx.send(response).unwrap();
    });

    block_on(subscribe_to_orderbook_topic(&ctx, "RICK", "MORTY", true)).unwrap();

    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());

    let actual = orderbook
        .topics_subscribed_to
        .get(&orderbook_topic("RICK", "MORTY"))
        .cloned();
    let expected = Some(OrderbookRequestingState::Requested);
    assert_eq!(actual, expected);
}

#[test]
fn test_subscribe_to_ordermatch_topic_subscribed_not_filled() {
    let (ctx, _pubkey, _secret) = make_ctx_for_tests();
    let (_, mut cmd_rx) = p2p_context_mock();

    {
        let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
        let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());
        // not enough time has passed for the orderbook to be filled
        let subscribed_at = now_ms() / 1000 - ORDERBOOK_REQUESTING_TIMEOUT + 1;
        orderbook.topics_subscribed_to.insert(
            orderbook_topic("RICK", "MORTY"),
            OrderbookRequestingState::NotRequested { subscribed_at },
        );
    }

    spawn(async move {
        let (req, response_tx) = match cmd_rx.next().await.unwrap() {
            AdexBehaviourCmd::RequestRelays { req, response_tx } => (req, response_tx),
            _ => panic!("AdexBehaviourCmd::RequestRelays expected"),
        };

        let request = decode_message::<P2PRequest>(&req).unwrap();
        match request {
            P2PRequest::Ordermatch(OrdermatchRequest::GetOrderbook { .. }) => (),
            _ => panic!(),
        }

        let response = new_protocol::Orderbook {
            asks: Vec::new(),
            bids: Vec::new(),
        };
        let encoded = encode_message(&response).unwrap();
        let response = vec![(PeerId::random(), AdexResponse::Ok { response: encoded })];
        response_tx.send(response).unwrap();
    });

    block_on(subscribe_to_orderbook_topic(&ctx, "RICK", "MORTY", true)).unwrap();

    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());

    let actual = orderbook
        .topics_subscribed_to
        .get(&orderbook_topic("RICK", "MORTY"))
        .cloned();
    let expected = Some(OrderbookRequestingState::Requested);
    assert_eq!(actual, expected);

    // orderbook.topics_subscribed_to.insert(orderbook_topic("RICK", "MORTY"), OrderbookSubscriptionState::NotRequested {subscribed_at: now_ms() - 41});
}

#[test]
fn test_subscribe_to_ordermatch_topic_subscribed_filled() {
    let (ctx, _pubkey, _secret) = make_ctx_for_tests();
    let (_, mut cmd_rx) = p2p_context_mock();

    // enough time has passed for the orderbook to be filled
    let subscribed_at = now_ms() / 1000 - ORDERBOOK_REQUESTING_TIMEOUT - 1;
    {
        let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
        let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());
        orderbook.topics_subscribed_to.insert(
            orderbook_topic("RICK", "MORTY"),
            OrderbookRequestingState::NotRequested { subscribed_at },
        );
    }

    spawn(async move {
        assert!(cmd_rx.next().await.is_none(), "No commands expected");
    });

    block_on(subscribe_to_orderbook_topic(&ctx, "RICK", "MORTY", true)).unwrap();

    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());

    let actual = orderbook
        .topics_subscribed_to
        .get(&orderbook_topic("RICK", "MORTY"))
        .cloned();
    let expected = Some(OrderbookRequestingState::NotRequested { subscribed_at });
    assert_eq!(actual, expected);
}
*/
#[test]
fn test_taker_request_can_match_with_maker_pubkey() {
    let coin = TestCoin::default().into();

    let maker_pubkey = H256Json::default();

    // default has MatchBy::Any
    let mut order = TakerOrderBuilder::new(&coin, &coin).build_unchecked();
    assert!(order.request.can_match_with_maker_pubkey(&maker_pubkey));

    // the uuids of orders is checked in another method
    order.request.match_by = MatchBy::Orders(HashSet::new());
    assert!(order.request.can_match_with_maker_pubkey(&maker_pubkey));

    let mut set = HashSet::new();
    set.insert(maker_pubkey.clone());
    order.request.match_by = MatchBy::Pubkeys(set);
    assert!(order.request.can_match_with_maker_pubkey(&maker_pubkey));

    order.request.match_by = MatchBy::Pubkeys(HashSet::new());
    assert!(!order.request.can_match_with_maker_pubkey(&maker_pubkey));
}

#[test]
fn test_taker_request_can_match_with_uuid() {
    let uuid = Uuid::new_v4();
    let coin = MmCoinEnum::Test(TestCoin::default());

    // default has MatchBy::Any
    let mut order = TakerOrderBuilder::new(&coin, &coin).build_unchecked();
    assert!(order.request.can_match_with_uuid(&uuid));

    // the uuids of orders is checked in another method
    order.request.match_by = MatchBy::Pubkeys(HashSet::new());
    assert!(order.request.can_match_with_uuid(&uuid));

    let mut set = HashSet::new();
    set.insert(uuid);
    order.request.match_by = MatchBy::Orders(set);
    assert!(order.request.can_match_with_uuid(&uuid));

    order.request.match_by = MatchBy::Orders(HashSet::new());
    assert!(!order.request.can_match_with_uuid(&uuid));
}

#[test]
fn test_orderbook_insert_or_update_order() {
    let (_, pubkey, secret) = make_ctx_for_tests();
    let mut orderbook = Orderbook::default();
    let order = make_random_orders(pubkey, &secret, "C1".into(), "C2".into(), 1).remove(0);
    orderbook.insert_or_update_order_update_trie(order);
}

fn pair_trie_root_by_pub(ctx: &MmArc, pubkey: &str, pair: &str) -> H64 {
    let ordermatch_ctx = OrdermatchContext::from_ctx(ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());
    *orderbook
        .pubkeys_state
        .get(pubkey)
        .unwrap()
        .trie_roots
        .get(pair)
        .unwrap()
}

fn clone_orderbook_memory_db(ctx: &MmArc) -> MemoryDB<Blake2Hasher64> {
    let ordermatch_ctx = OrdermatchContext::from_ctx(ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());
    orderbook.memory_db.clone()
}

fn remove_order(ctx: &MmArc, uuid: Uuid) {
    let ordermatch_ctx = OrdermatchContext::from_ctx(ctx).unwrap();
    let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());
    orderbook.remove_order_trie_update(uuid);
}

#[test]
fn test_process_sync_pubkey_orderbook_state_after_new_orders_added() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let orders = make_random_orders(pubkey.clone(), &secret, "C1".into(), "C2".into(), 100);

    for order in orders {
        block_on(insert_or_update_order(&ctx, order));
    }

    let alb_ordered_pair = alb_ordered_pair("C1", "C2");
    let pair_trie_root = pair_trie_root_by_pub(&ctx, &pubkey, &alb_ordered_pair);

    let prev_pairs_state = HashMap::from_iter(iter::once((alb_ordered_pair.clone(), pair_trie_root)));

    let mut old_mem_db = clone_orderbook_memory_db(&ctx);

    let new_orders = make_random_orders(pubkey.clone(), &secret, "C1".into(), "C2".into(), 100);
    for order in new_orders {
        block_on(insert_or_update_order(&ctx, order.clone()));
    }

    let mut result = block_on(process_sync_pubkey_orderbook_state(
        ctx.clone(),
        pubkey.clone(),
        prev_pairs_state,
    ))
    .unwrap()
    .unwrap();

    // check pair trie root
    let expected_root_hash = pair_trie_root_by_pub(&ctx, &pubkey, &alb_ordered_pair);

    let delta = match result.pair_orders_diff.remove(&alb_ordered_pair).unwrap() {
        DeltaOrFullTrie::Delta(delta) => delta,
        DeltaOrFullTrie::FullTrie(_) => panic!("Must be DeltaOrFullTrie::Delta"),
    };

    let actual_root_hash = delta_trie_root::<Layout, _, _, _, _, _>(
        &mut old_mem_db,
        pair_trie_root,
        delta
            .into_iter()
            .map(|(uuid, order)| (*uuid.as_bytes(), order.map(|o| encode_message(&o).unwrap()))),
    )
    .unwrap();
    assert_eq!(expected_root_hash, actual_root_hash);
}

#[test]
fn test_diff_should_not_be_written_if_hash_not_changed_on_insert() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let orders = make_random_orders(pubkey.clone(), &secret, "C1".into(), "C2".into(), 100);

    for order in orders.clone() {
        block_on(insert_or_update_order(&ctx, order));
    }

    let alb_ordered_pair = alb_ordered_pair("C1", "C2");
    let pair_trie_root = pair_trie_root_by_pub(&ctx, &pubkey, &alb_ordered_pair);
    for order in orders.clone() {
        block_on(insert_or_update_order(&ctx, order));
    }

    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());
    let pubkey_state = orderbook.pubkeys_state.get(&pubkey).unwrap();
    assert!(!pubkey_state
        .order_pairs_trie_state_history
        .get(&alb_ordered_pair)
        .expect("Must contain C1:C2 pair")
        .contains_key(&pair_trie_root));
}

#[test]
fn test_process_sync_pubkey_orderbook_state_after_orders_removed() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let orders = make_random_orders(pubkey.clone(), &secret, "C1".into(), "C2".into(), 100);

    for order in orders.clone() {
        block_on(insert_or_update_order(&ctx, order));
    }

    let alb_ordered_pair = alb_ordered_pair("C1", "C2");
    let pair_trie_root = pair_trie_root_by_pub(&ctx, &pubkey, &alb_ordered_pair);

    let prev_pairs_state = HashMap::from_iter(iter::once((alb_ordered_pair.clone(), pair_trie_root)));

    let mut old_mem_db = clone_orderbook_memory_db(&ctx);

    // pick 10 orders at random and remove them
    let mut rng = thread_rng();
    let to_remove = orders.choose_multiple(&mut rng, 10);
    for order in to_remove {
        remove_order(&ctx, order.uuid);
    }

    let mut result = block_on(process_sync_pubkey_orderbook_state(
        ctx.clone(),
        pubkey.clone(),
        prev_pairs_state,
    ))
    .unwrap()
    .unwrap();

    // check pair trie root
    let expected_root_hash = pair_trie_root_by_pub(&ctx, &pubkey, &alb_ordered_pair);

    let delta = match result.pair_orders_diff.remove(&alb_ordered_pair).unwrap() {
        DeltaOrFullTrie::Delta(delta) => delta,
        DeltaOrFullTrie::FullTrie(_) => panic!("Must be DeltaOrFullTrie::Delta"),
    };

    let actual_root_hash = delta_trie_root::<Layout, _, _, _, _, _>(
        &mut old_mem_db,
        pair_trie_root,
        delta
            .into_iter()
            .map(|(uuid, order)| (*uuid.as_bytes(), order.map(|o| encode_message(&o).unwrap()))),
    )
    .unwrap();
    assert_eq!(expected_root_hash, actual_root_hash);
}

#[test]
fn test_diff_should_not_be_written_if_hash_not_changed_on_remove() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let orders = make_random_orders(pubkey.clone(), &secret, "C1".into(), "C2".into(), 100);

    for order in orders.clone() {
        block_on(insert_or_update_order(&ctx, order));
    }

    let to_remove: Vec<_> = orders
        .choose_multiple(&mut thread_rng(), 10)
        .map(|order| order.uuid)
        .collect();
    for uuid in &to_remove {
        remove_order(&ctx, *uuid);
    }
    for uuid in &to_remove {
        remove_order(&ctx, *uuid);
    }

    let alb_ordered_pair = alb_ordered_pair("C1", "C2");
    let pair_trie_root = pair_trie_root_by_pub(&ctx, &pubkey, &alb_ordered_pair);

    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    let orderbook = block_on(ordermatch_ctx.orderbook.lock());
    let pubkey_state = orderbook.pubkeys_state.get(&pubkey).unwrap();
    assert!(!pubkey_state
        .order_pairs_trie_state_history
        .get(&alb_ordered_pair)
        .expect("Must contain C1:C2 pair")
        .contains_key(&pair_trie_root));
}

#[test]
fn test_orderbook_pubkey_sync_request() {
    let mut orderbook = Orderbook::default();
    orderbook.topics_subscribed_to.insert(
        orderbook_topic_from_base_rel("C1", "C2"),
        OrderbookRequestingState::Requested,
    );
    let pubkey = "pubkey";

    let mut trie_roots = HashMap::new();
    trie_roots.insert("C1:C2".to_owned(), [1; 8]);
    trie_roots.insert("C2:C3".to_owned(), [1; 8]);

    let message = PubkeyKeepAlive {
        trie_roots,
        timestamp: now_ms() / 1000,
    };

    let request = orderbook.process_keep_alive(pubkey, message, false).unwrap();
    match request {
        OrdermatchRequest::SyncPubkeyOrderbookState {
            trie_roots: pairs_trie_roots,
            ..
        } => {
            assert!(pairs_trie_roots.contains_key("C1:C2"));
            assert!(!pairs_trie_roots.contains_key("C2:C3"));
        },
        _ => panic!("Invalid request {:?}", request),
    }
}

#[test]
fn test_orderbook_pubkey_sync_request_relay() {
    let mut orderbook = Orderbook::default();
    orderbook.topics_subscribed_to.insert(
        orderbook_topic_from_base_rel("C1", "C2"),
        OrderbookRequestingState::Requested,
    );
    let pubkey = "pubkey";

    let mut trie_roots = HashMap::new();
    trie_roots.insert("C1:C2".to_owned(), [1; 8]);
    trie_roots.insert("C2:C3".to_owned(), [1; 8]);

    let message = PubkeyKeepAlive {
        trie_roots,
        timestamp: now_ms() / 1000,
    };

    let request = orderbook.process_keep_alive(pubkey, message, true).unwrap();
    match request {
        OrdermatchRequest::SyncPubkeyOrderbookState {
            trie_roots: pairs_trie_roots,
            ..
        } => {
            assert!(pairs_trie_roots.contains_key("C1:C2"));
            assert!(pairs_trie_roots.contains_key("C2:C3"));
        },
        _ => panic!("Invalid request {:?}", request),
    }
}

#[test]
fn test_trie_diff_avoid_cycle_on_insertion() {
    let mut history = TrieDiffHistory::<String, String>::default();
    history.insert_new_diff([1; 8], TrieDiff {
        delta: vec![],
        next_root: [2; 8],
    });

    history.insert_new_diff([2; 8], TrieDiff {
        delta: vec![],
        next_root: [3; 8],
    });

    history.insert_new_diff([3; 8], TrieDiff {
        delta: vec![],
        next_root: [4; 8],
    });

    history.insert_new_diff([4; 8], TrieDiff {
        delta: vec![],
        next_root: [5; 8],
    });

    history.insert_new_diff([5; 8], TrieDiff {
        delta: vec![],
        next_root: [2; 8],
    });

    let expected = TrieDiffHistory {
        inner: HashMap::from_iter(iter::once(([1; 8], TrieDiff {
            delta: vec![],
            next_root: [2; 8],
        }))),
    };

    assert_eq!(expected, history);
}

#[test]
fn test_process_sync_pubkey_orderbook_state_points_to_not_uptodate_trie_root() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let orders = make_random_orders(pubkey.clone(), &secret, "RICK".into(), "MORTY".into(), 10);
    let new_order = make_random_orders(pubkey.clone(), &secret, "RICK".into(), "MORTY".into(), 1)
        .pop()
        .expect("Expected one order");

    for order in orders.iter() {
        block_on(insert_or_update_order(&ctx, order.clone()));
    }

    let alb_pair = alb_ordered_pair("RICK", "MORTY");

    // update trie root by adding a new order and do not update history
    let (old_root, _new_root) = {
        let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
        let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());

        log!([pubkey]", found "[orderbook.pubkeys_state.keys()]);
        let old_root = *orderbook
            .pubkeys_state
            .get_mut(&pubkey)
            .expect("!pubkeys_state")
            .trie_roots
            .get(&alb_pair)
            .expect("MORTY:RICK must be in trie_roots");

        let order_bytes = rmp_serde::to_vec(&new_order).expect("Serialization should never fail");
        let mut new_root = old_root;
        let mut trie = get_trie_mut(&mut orderbook.memory_db, &mut new_root).expect("!get_trie_mut");
        trie.insert(new_order.uuid.as_bytes(), &order_bytes)
            .expect("Error on order insertion");
        drop(trie);

        // update root in orderbook trie_roots
        orderbook
            .pubkeys_state
            .get_mut(&pubkey)
            .expect("!pubkeys_state")
            .trie_roots
            .insert(alb_pair.clone(), new_root);

        (old_root, new_root)
    };

    let mut roots = HashMap::new();
    roots.insert(alb_pair.clone(), old_root);

    let SyncPubkeyOrderbookStateRes {
        mut pair_orders_diff, ..
    } = block_on(process_sync_pubkey_orderbook_state(ctx, pubkey, roots))
        .expect("!process_sync_pubkey_orderbook_state")
        .expect("Expected MORTY:RICK delta, returned None");

    let delta = pair_orders_diff.remove(&alb_pair).expect("Expected MORTY:RICK delta");
    let mut full_trie = match delta {
        DeltaOrFullTrie::Delta(_) => panic!("Expected FullTrie, found Delta"),
        DeltaOrFullTrie::FullTrie(full_trie) => full_trie,
    };

    let mut expected: Vec<_> = orders.into_iter().map(|order| (order.uuid, order)).collect();
    expected.push((new_order.uuid, new_order));
    full_trie.sort_by(|x, y| x.0.cmp(&y.0));
    expected.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(full_trie, expected);
}

fn check_if_orderbook_contains_only(orderbook: &Orderbook, pubkey: &str, orders: &Vec<OrderbookItem>) {
    let pubkey_state = orderbook.pubkeys_state.get(pubkey).expect("!pubkeys_state");

    // order_set
    let expected_set: HashMap<_, _> = orders.iter().map(|order| (order.uuid, order.clone())).collect();
    assert_eq!(orderbook.order_set, expected_set);

    // ordered
    let mut expected_ordered = HashMap::new();
    for order in orders.iter() {
        let item = OrderedByPriceOrder {
            uuid: order.uuid,
            price: order.price.clone().into(),
        };
        let set = expected_ordered
            .entry((order.base.clone(), order.rel.clone()))
            .or_insert_with(BTreeSet::default);
        set.insert(item);
    }
    assert_eq!(orderbook.ordered, expected_ordered);

    // unordered
    let mut expected_unordered = HashMap::new();
    for order in orders.iter() {
        let set = expected_unordered
            .entry((order.base.clone(), order.rel.clone()))
            .or_insert_with(HashSet::default);
        set.insert(order.uuid);
    }
    assert_eq!(orderbook.unordered, expected_unordered);

    // history
    let actual_keys: HashSet<_> = pubkey_state.order_pairs_trie_state_history.keys().cloned().collect();
    let expected_keys: HashSet<_> = orders
        .iter()
        .map(|order| alb_ordered_pair(&order.base, &order.rel))
        .collect();
    assert_eq!(actual_keys, expected_keys);

    // orders_uuids
    let expected_uuids: HashSet<_> = orders
        .iter()
        .map(|order| (order.uuid, alb_ordered_pair(&order.base, &order.rel)))
        .collect();
    assert_eq!(pubkey_state.orders_uuids, expected_uuids);

    // trie_roots
    let actual_trie_orders: HashMap<_, _> = pubkey_state
        .trie_roots
        .iter()
        .map(|(alb_pair, trie_root)| {
            let trie = TrieDB::<Layout>::new(&orderbook.memory_db, trie_root).expect("!TrieDB::new");
            let mut trie: Vec<(Uuid, OrderbookItem)> = trie
                .iter()
                .expect("!TrieDB::iter")
                .map(|key_value| {
                    let (key, value) = key_value.expect("Iterator returned an error");
                    let key = TryFromBytes::try_from_bytes(key).expect("!try_from_bytes() key");
                    let value = TryFromBytes::try_from_bytes(value).expect("!try_from_bytes() val");
                    (key, value)
                })
                .collect();
            trie.sort_by(|(uuid_x, _), (uuid_y, _)| uuid_x.cmp(uuid_y));
            (alb_pair.clone(), trie)
        })
        .collect();
    let mut expected_trie_orders = HashMap::new();
    for order in orders.iter() {
        let trie = expected_trie_orders
            .entry(alb_ordered_pair(&order.base, &order.rel))
            .or_insert_with(Vec::default);
        trie.push((order.uuid, order.clone()));
    }
    for (_alb_pair, trie) in expected_trie_orders.iter_mut() {
        trie.sort_by(|(uuid_x, _), (uuid_y, _)| uuid_x.cmp(uuid_y));
    }
    assert_eq!(actual_trie_orders, expected_trie_orders);
}

#[test]
fn test_remove_and_purge_pubkey_pair_orders() {
    let (ctx, pubkey, secret) = make_ctx_for_tests();
    let rick_morty_orders = make_random_orders(pubkey.clone(), &secret, "RICK".into(), "MORTY".into(), 10);
    let rick_kmd_orders = make_random_orders(pubkey.clone(), &secret, "RICK".into(), "KMD".into(), 10);

    for order in rick_morty_orders.iter().chain(rick_kmd_orders.iter()) {
        block_on(insert_or_update_order(&ctx, order.clone()));
    }

    let rick_morty_pair = alb_ordered_pair("RICK", "MORTY");

    let ordermatch_ctx = OrdermatchContext::from_ctx(&ctx).unwrap();
    let mut orderbook = block_on(ordermatch_ctx.orderbook.lock());

    remove_and_purge_pubkey_pair_orders(&mut orderbook, &pubkey, &rick_morty_pair);
    check_if_orderbook_contains_only(&orderbook, &pubkey, &rick_kmd_orders);
}
