use crate::utxo::rpc_clients::{UnspentInfo, UtxoRpcClientEnum, UtxoRpcClientOps, UtxoRpcError, UtxoRpcFut,
                               UtxoRpcResult};
use crate::utxo::utxo_common::{big_decimal_from_sat_unsigned, payment_script, UtxoArcBuilder};
use crate::utxo::{sat_from_big_decimal, utxo_common, ActualTxFee, AdditionalTxData, Address, FeePolicy, HistoryUtxoTx,
                  HistoryUtxoTxMap, RecentlySpentOutPoints, UtxoArc, UtxoCoinBuilder, UtxoCoinFields, UtxoCommonOps,
                  VerboseTransactionFrom};
use crate::{BalanceFut, CoinBalance, FeeApproxStage, FoundSwapTxSpend, HistorySyncState, MarketCoinOps, MmCoin,
            NegotiateSwapContractAddrErr, NumConversError, SwapOps, TradeFee, TradePreimageFut, TradePreimageResult,
            TradePreimageValue, TransactionEnum, TransactionFut, ValidateAddressResult, WithdrawFut, WithdrawRequest};
use async_trait::async_trait;
use bitcrypto::dhash160;
use chain::constants::SEQUENCE_FINAL;
use chain::{Transaction as UtxoTx, TransactionOutput};
use common::jsonrpc_client::JsonRpcError;
use common::mm_ctx::MmArc;
use common::mm_error::prelude::*;
use common::mm_number::{BigDecimal, MmNumber};
use common::now_ms;
use derive_more::Display;
use futures::compat::Future01CompatExt;
use futures::lock::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};
use futures::{FutureExt, TryFutureExt};
use futures01::Future;
use keys::hash::H256;
use keys::Public;
use primitives::bytes::Bytes;
use rpc::v1::types::{Bytes as BytesJson, Transaction as RpcTransaction, H256 as H256Json};
use script::{Builder as ScriptBuilder, Opcode, Script, TransactionInputSigner};
use serde_json::Value as Json;
use serialization::{deserialize, CoinVariant};
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use zcash_client_backend::decrypt_transaction;
use zcash_client_backend::encoding::{encode_extended_spending_key, encode_payment_address};
use zcash_client_backend::wallet::AccountId;
use zcash_primitives::legacy::Script as ZCashScript;
use zcash_primitives::merkle_tree::{CommitmentTree, Hashable, IncrementalWitness};
use zcash_primitives::sapling::Node;
use zcash_primitives::transaction::builder::{Builder as ZTxBuilder, Error as ZTxBuilderError};
use zcash_primitives::transaction::components::{Amount, TxOut};
use zcash_primitives::transaction::Transaction as ZTransaction;
use zcash_primitives::{consensus, constants::mainnet as z_mainnet_constants, sapling::PaymentAddress,
                       zip32::ExtendedSpendingKey};
use zcash_proofs::prover::LocalTxProver;

mod z_htlc;
use z_htlc::{z_p2sh_spend, z_send_dex_fee, z_send_htlc};

mod z_rpc;
use crate::z_coin::z_rpc::ZUnspent;
use z_rpc::ZRpcOps;

#[cfg(test)] mod z_coin_tests;

pub struct ZCoinFields {
    z_spending_key: ExtendedSpendingKey,
    z_addr: PaymentAddress,
    z_addr_encoded: String,
    z_tx_prover: LocalTxProver,
    /// Mutex preventing concurrent transaction generation/same input usage
    z_tx_mutex: AsyncMutex<()>,
}

impl std::fmt::Debug for ZCoinFields {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "ZCoinFields {{ z_addr: {:?}, z_addr_encoded: {} }}",
            self.z_addr, self.z_addr_encoded
        )
    }
}

#[derive(Clone, Debug)]
pub struct ZCoin {
    utxo_arc: UtxoArc,
    z_fields: Arc<ZCoinFields>,
}

#[derive(Debug, Display)]
#[allow(clippy::large_enum_variant)]
pub enum SendOutputsErr {
    TxBuilderError(ZTxBuilderError),
    Rpc(UtxoRpcError),
    NumConversion(NumConversError),
    #[display(
        fmt = "Not enough {} to generate a tx: available {}, required at least {}",
        coin,
        available,
        required
    )]
    InsufficientBalance {
        coin: String,
        available: BigDecimal,
        required: BigDecimal,
    },
}

impl From<ZTxBuilderError> for SendOutputsErr {
    fn from(err: ZTxBuilderError) -> SendOutputsErr { SendOutputsErr::TxBuilderError(err) }
}

impl From<UtxoRpcError> for SendOutputsErr {
    fn from(err: UtxoRpcError) -> SendOutputsErr { SendOutputsErr::Rpc(err) }
}

impl From<NumConversError> for SendOutputsErr {
    fn from(err: NumConversError) -> SendOutputsErr { SendOutputsErr::NumConversion(err) }
}

impl ZCoin {
    pub fn z_rpc(&self) -> &(dyn ZRpcOps + Send + Sync) { self.utxo_arc.rpc_client.as_ref() }

    pub fn rpc_client(&self) -> &UtxoRpcClientEnum { &self.utxo_arc.rpc_client }

    async fn my_z_unspents_ordered(&self) -> UtxoRpcResult<Vec<ZUnspent>> {
        let mut unspents = self
            .z_rpc()
            .z_list_unspent(0, i32::MAX as u32, true, &[&self.z_fields.z_addr_encoded])
            .compat()
            .await?;

        unspents.sort_unstable_by(|a, b| a.amount.cmp(&b.amount));
        Ok(unspents)
    }

    pub async fn send_outputs(&self, outputs: Vec<TransactionOutput>) -> Result<UtxoTx, MmError<SendOutputsErr>> {
        let _lock = self.z_fields.z_tx_mutex.lock().await;
        let tx_fee: BigDecimal = "0.00001".parse().unwrap();
        let total_output_sat = outputs.iter().fold(0, |cur, out| cur + out.value);
        let total_output = big_decimal_from_sat_unsigned(total_output_sat, self.utxo_arc.decimals);
        let total_required = &total_output + &tx_fee;

        let z_unspents = self.my_z_unspents_ordered().await?;
        let mut selected_unspents = Vec::new();
        let mut total_input_amount = BigDecimal::from(0);
        let mut change = BigDecimal::from(0);

        for unspent in z_unspents {
            total_input_amount += unspent.amount.to_decimal();
            selected_unspents.push(unspent);

            if total_input_amount >= total_required {
                change = &total_input_amount - &total_required;
                break;
            }
        }

        if total_input_amount < total_required {
            return MmError::err(SendOutputsErr::InsufficientBalance {
                coin: self.ticker().into(),
                available: total_input_amount,
                required: total_required,
            });
        }

        let current_block = self.utxo_arc.rpc_client.get_block_count().compat().await? as u32;
        let mut tx_builder = ZTxBuilder::new(consensus::MAIN_NETWORK, current_block.into());

        let mut ext = HashMap::new();
        ext.insert(AccountId::default(), (&self.z_fields.z_spending_key).into());
        let mut selected_notes_with_witness: Vec<(_, Option<IncrementalWitness<Node>>)> =
            Vec::with_capacity(selected_unspents.len());

        for unspent in selected_unspents {
            let prev_tx = self
                .rpc_client()
                .get_verbose_transaction(unspent.txid.clone())
                .compat()
                .await?;

            let z_cash_tx = ZTransaction::read(prev_tx.hex.as_slice()).unwrap();
            let decrypted = decrypt_transaction(
                &consensus::MAIN_NETWORK,
                prev_tx.height.unwrap().try_into().unwrap(),
                &z_cash_tx,
                &ext,
            );
            let decrypted_output = decrypted
                .iter()
                .find(|out| out.index as u32 == unspent.out_index)
                .unwrap();
            selected_notes_with_witness.push((decrypted_output.note.clone(), None));
        }

        let mut tree = CommitmentTree::<Node>::empty();
        let mut root = [0u8; 32];
        tree.root().write(&mut root as &mut [u8]).unwrap();
        let default_root = Some(H256Json::from(root).reversed());

        let mut processed_block = 0;
        let native_client = match self.rpc_client() {
            UtxoRpcClientEnum::Native(native) => native,
            _ => panic!(),
        };

        let current_block = native_client.get_block_count().compat().await?;
        let mut current_sapling_root = default_root;
        let zero_root = Some(H256Json::default());

        while processed_block <= current_block {
            let block_hash = native_client.get_block_hash(processed_block).compat().await?;
            let block = native_client.get_block(block_hash).compat().await?;
            if current_sapling_root != block.final_sapling_root && block.final_sapling_root != zero_root {
                current_sapling_root = block.final_sapling_root;
                for hash in block.tx {
                    let tx = native_client.get_transaction_bytes(hash).compat().await?;
                    let tx: UtxoTx = deserialize(tx.as_slice()).unwrap();
                    for output in tx.shielded_outputs {
                        for (_, witness) in selected_notes_with_witness.iter_mut() {
                            if let Some(ref mut w) = witness {
                                w.append(Node::new(output.cmu.clone().take())).unwrap();
                            }
                        }

                        tree.append(Node::new(output.cmu.clone().take())).unwrap();

                        for (note, witness) in selected_notes_with_witness.iter_mut() {
                            if H256::from(note.cmu().to_bytes()) == output.cmu {
                                *witness = Some(IncrementalWitness::from_tree(&tree));
                            }
                        }
                    }
                }
            }
            processed_block += 1;
        }

        for (note, witness) in selected_notes_with_witness {
            tx_builder.add_sapling_spend(
                self.z_fields.z_spending_key.clone(),
                *self.z_fields.z_addr.diversifier(),
                note,
                witness.unwrap().path().unwrap(),
            )?;
        }

        if change > BigDecimal::from(0) {
            let change_sat = sat_from_big_decimal(&change, self.utxo_arc.decimals)?;

            tx_builder.add_sapling_output(
                None,
                self.z_fields.z_addr.clone(),
                Amount::from_u64(change_sat).unwrap(),
                None,
            )?;
        }

        for output in outputs {
            let z_cash_out = TxOut {
                value: Amount::from_u64(output.value).unwrap(),
                script_pubkey: ZCashScript(output.script_pubkey.take()),
            };
            tx_builder.add_tx_out(z_cash_out);
        }

        let (tx, _) = tx_builder.build(consensus::BranchId::Sapling, &self.z_fields.z_tx_prover)?;
        let mut tx_buffer = Vec::with_capacity(1024);
        tx.write(&mut tx_buffer).unwrap();

        let mm_tx: UtxoTx = deserialize(tx_buffer.as_slice()).expect("librustzcash should produce a valid tx");

        self.rpc_client()
            .send_raw_transaction(tx_buffer.into())
            .compat()
            .await?;

        Ok(mm_tx)
    }
}

impl AsRef<UtxoCoinFields> for ZCoin {
    fn as_ref(&self) -> &UtxoCoinFields { &self.utxo_arc }
}

#[derive(Debug, Display)]
pub enum ZCoinBuildError {
    BuilderError(String),
    GetAddressError,
}

pub async fn z_coin_from_conf_and_request(
    ctx: &MmArc,
    ticker: &str,
    conf: &Json,
    req: &Json,
    secp_priv_key: &[u8],
) -> Result<ZCoin, MmError<ZCoinBuildError>> {
    let z_key = ExtendedSpendingKey::master(secp_priv_key);
    z_coin_from_conf_and_request_with_z_key(ctx, ticker, conf, req, secp_priv_key, z_key).await
}

async fn z_coin_from_conf_and_request_with_z_key(
    ctx: &MmArc,
    ticker: &str,
    conf: &Json,
    req: &Json,
    secp_priv_key: &[u8],
    z_spending_key: ExtendedSpendingKey,
) -> Result<ZCoin, MmError<ZCoinBuildError>> {
    let builder = UtxoArcBuilder::new(ctx, ticker, conf, req, secp_priv_key);
    let utxo_arc = builder
        .build()
        .await
        .map_err(|e| MmError::new(ZCoinBuildError::BuilderError(e)))?;

    let (_, z_addr) = z_spending_key
        .default_address()
        .map_err(|_| MmError::new(ZCoinBuildError::GetAddressError))?;

    let z_tx_prover = LocalTxProver::bundled();
    let z_addr_encoded = encode_payment_address(z_mainnet_constants::HRP_SAPLING_PAYMENT_ADDRESS, &z_addr);
    let z_fields = ZCoinFields {
        z_spending_key,
        z_addr,
        z_addr_encoded,
        z_tx_prover,
        z_tx_mutex: AsyncMutex::new(()),
    };
    Ok(ZCoin {
        utxo_arc,
        z_fields: Arc::new(z_fields),
    })
}

impl MarketCoinOps for ZCoin {
    fn ticker(&self) -> &str { &self.utxo_arc.conf.ticker }

    fn my_address(&self) -> Result<String, String> { Ok(self.z_fields.z_addr_encoded.clone()) }

    fn my_balance(&self) -> BalanceFut<CoinBalance> {
        let min_conf = 0;
        let fut = self
            .utxo_arc
            .rpc_client
            .as_ref()
            .z_get_balance(&self.z_fields.z_addr_encoded, min_conf)
            // at the moment Z coins do not have an unspendable balance
            .map(|spendable| CoinBalance {
                spendable: spendable.to_decimal(),
                unspendable: BigDecimal::from(0),
            })
            .map_err(|e| e.into());
        Box::new(fut)
    }

    fn base_coin_balance(&self) -> BalanceFut<BigDecimal> { utxo_common::base_coin_balance(self) }

    fn send_raw_tx(&self, tx: &str) -> Box<dyn Future<Item = String, Error = String> + Send> {
        utxo_common::send_raw_tx(self.as_ref(), tx)
    }

    fn wait_for_confirmations(
        &self,
        tx: &[u8],
        confirmations: u64,
        requires_nota: bool,
        wait_until: u64,
        check_every: u64,
    ) -> Box<dyn Future<Item = (), Error = String> + Send> {
        utxo_common::wait_for_confirmations(self.as_ref(), tx, confirmations, requires_nota, wait_until, check_every)
    }

    fn wait_for_tx_spend(
        &self,
        transaction: &[u8],
        wait_until: u64,
        from_block: u64,
        _swap_contract_address: &Option<BytesJson>,
    ) -> TransactionFut {
        utxo_common::wait_for_output_spend(
            self.as_ref(),
            transaction,
            utxo_common::DEFAULT_SWAP_VOUT,
            from_block,
            wait_until,
        )
    }

    fn tx_enum_from_bytes(&self, bytes: &[u8]) -> Result<TransactionEnum, String> {
        utxo_common::tx_enum_from_bytes(self.as_ref(), bytes)
    }

    fn current_block(&self) -> Box<dyn Future<Item = u64, Error = String> + Send> {
        utxo_common::current_block(&self.utxo_arc)
    }

    fn address_from_pubkey_str(&self, _pubkey: &str) -> Result<String, String> { todo!() }

    fn display_priv_key(&self) -> String {
        encode_extended_spending_key(
            z_mainnet_constants::HRP_SAPLING_EXTENDED_SPENDING_KEY,
            &self.z_fields.z_spending_key,
        )
    }

    fn min_tx_amount(&self) -> BigDecimal { utxo_common::min_tx_amount(self.as_ref()) }

    fn min_trading_vol(&self) -> MmNumber { utxo_common::min_trading_vol(self.as_ref()) }
}

impl SwapOps for ZCoin {
    fn send_taker_fee(&self, _fee_addr: &[u8], amount: BigDecimal) -> TransactionFut {
        // TODO replace dummy locktime and watcher pub
        let selfi = self.clone();
        let fut = async move {
            let (tx, _) = try_s!(
                z_send_dex_fee(
                    &selfi,
                    (now_ms() / 1000) as u32,
                    selfi.utxo_arc.key_pair.public(),
                    amount
                )
                .await
            );
            Ok(tx.into())
        };
        Box::new(fut.boxed().compat())
    }

    fn send_maker_payment(
        &self,
        time_lock: u32,
        taker_pub: &[u8],
        secret_hash: &[u8],
        amount: BigDecimal,
        _swap_contract_address: &Option<BytesJson>,
    ) -> TransactionFut {
        let selfi = self.clone();
        let taker_pub = try_fus!(Public::from_slice(taker_pub));
        let secret_hash = secret_hash.to_vec();
        let fut = async move {
            let utxo_tx = try_s!(z_send_htlc(&selfi, time_lock, &taker_pub, &secret_hash, amount).await);
            Ok(utxo_tx.into())
        };
        Box::new(fut.boxed().compat())
    }

    fn send_taker_payment(
        &self,
        time_lock: u32,
        maker_pub: &[u8],
        secret_hash: &[u8],
        amount: BigDecimal,
        _swap_contract_address: &Option<BytesJson>,
    ) -> TransactionFut {
        let selfi = self.clone();
        let maker_pub = try_fus!(Public::from_slice(maker_pub));
        let secret_hash = secret_hash.to_vec();
        let fut = async move {
            let utxo_tx = try_s!(z_send_htlc(&selfi, time_lock, &maker_pub, &secret_hash, amount).await);
            Ok(utxo_tx.into())
        };
        Box::new(fut.boxed().compat())
    }

    fn send_maker_spends_taker_payment(
        &self,
        taker_payment_tx: &[u8],
        time_lock: u32,
        taker_pub: &[u8],
        secret: &[u8],
        _swap_contract_address: &Option<BytesJson>,
    ) -> TransactionFut {
        let tx: UtxoTx = try_fus!(deserialize(taker_payment_tx).map_err(|e| ERRL!("{:?}", e)));
        let redeem_script = payment_script(
            time_lock,
            &*dhash160(secret),
            &Public::from_slice(taker_pub).unwrap(),
            self.utxo_arc.key_pair.public(),
        );
        let script_data = ScriptBuilder::default()
            .push_data(secret)
            .push_opcode(Opcode::OP_0)
            .into_script();
        let selfi = self.clone();
        let fut = async move {
            let tx_fut = z_p2sh_spend(&selfi, tx, time_lock, SEQUENCE_FINAL, redeem_script, script_data);
            let tx = try_s!(tx_fut.await);
            Ok(tx.into())
        };
        Box::new(fut.boxed().compat())
    }

    fn send_taker_spends_maker_payment(
        &self,
        maker_payment_tx: &[u8],
        time_lock: u32,
        maker_pub: &[u8],
        secret: &[u8],
        _swap_contract_address: &Option<BytesJson>,
    ) -> TransactionFut {
        let tx: UtxoTx = try_fus!(deserialize(maker_payment_tx).map_err(|e| ERRL!("{:?}", e)));
        let redeem_script = payment_script(
            time_lock,
            &*dhash160(secret),
            &Public::from_slice(maker_pub).unwrap(),
            self.utxo_arc.key_pair.public(),
        );
        let script_data = ScriptBuilder::default()
            .push_data(secret)
            .push_opcode(Opcode::OP_0)
            .into_script();
        let selfi = self.clone();
        let fut = async move {
            let tx_fut = z_p2sh_spend(&selfi, tx, time_lock, SEQUENCE_FINAL, redeem_script, script_data);
            let tx = try_s!(tx_fut.await);
            Ok(tx.into())
        };
        Box::new(fut.boxed().compat())
    }

    fn send_taker_refunds_payment(
        &self,
        taker_payment_tx: &[u8],
        time_lock: u32,
        maker_pub: &[u8],
        secret_hash: &[u8],
        _swap_contract_address: &Option<BytesJson>,
    ) -> TransactionFut {
        let tx: UtxoTx = try_fus!(deserialize(taker_payment_tx).map_err(|e| ERRL!("{:?}", e)));
        let redeem_script = payment_script(
            time_lock,
            secret_hash,
            self.utxo_arc.key_pair.public(),
            &Public::from_slice(maker_pub).unwrap(),
        );
        let script_data = ScriptBuilder::default().push_opcode(Opcode::OP_1).into_script();
        let selfi = self.clone();
        let fut = async move {
            let tx_fut = z_p2sh_spend(&selfi, tx, time_lock, SEQUENCE_FINAL - 1, redeem_script, script_data);
            let tx = try_s!(tx_fut.await);
            Ok(tx.into())
        };
        Box::new(fut.boxed().compat())
    }

    fn send_maker_refunds_payment(
        &self,
        maker_payment_tx: &[u8],
        time_lock: u32,
        taker_pub: &[u8],
        secret_hash: &[u8],
        _swap_contract_address: &Option<BytesJson>,
    ) -> TransactionFut {
        let tx: UtxoTx = try_fus!(deserialize(maker_payment_tx).map_err(|e| ERRL!("{:?}", e)));
        let redeem_script = payment_script(
            time_lock,
            secret_hash,
            self.utxo_arc.key_pair.public(),
            &Public::from_slice(taker_pub).unwrap(),
        );
        let script_data = ScriptBuilder::default().push_opcode(Opcode::OP_1).into_script();
        let selfi = self.clone();
        let fut = async move {
            let tx_fut = z_p2sh_spend(&selfi, tx, time_lock, SEQUENCE_FINAL - 1, redeem_script, script_data);
            let tx = try_s!(tx_fut.await);
            Ok(tx.into())
        };
        Box::new(fut.boxed().compat())
    }

    fn validate_fee(
        &self,
        _fee_tx: &TransactionEnum,
        _expected_sender: &[u8],
        _fee_addr: &[u8],
        _amount: &BigDecimal,
        _min_block_number: u64,
    ) -> Box<dyn Future<Item = (), Error = String> + Send> {
        // TODO avoid dummy implementation
        Box::new(futures01::future::ok(()))
    }

    fn validate_maker_payment(
        &self,
        payment_tx: &[u8],
        time_lock: u32,
        maker_pub: &[u8],
        priv_bn_hash: &[u8],
        amount: BigDecimal,
        _swap_contract_address: &Option<BytesJson>,
    ) -> Box<dyn Future<Item = (), Error = String> + Send> {
        utxo_common::validate_maker_payment(self, payment_tx, time_lock, maker_pub, priv_bn_hash, amount)
    }

    fn validate_taker_payment(
        &self,
        payment_tx: &[u8],
        time_lock: u32,
        taker_pub: &[u8],
        priv_bn_hash: &[u8],
        amount: BigDecimal,
        _swap_contract_address: &Option<BytesJson>,
    ) -> Box<dyn Future<Item = (), Error = String> + Send> {
        utxo_common::validate_taker_payment(self, payment_tx, time_lock, taker_pub, priv_bn_hash, amount)
    }

    fn check_if_my_payment_sent(
        &self,
        time_lock: u32,
        other_pub: &[u8],
        secret_hash: &[u8],
        _search_from_block: u64,
        _swap_contract_address: &Option<BytesJson>,
    ) -> Box<dyn Future<Item = Option<TransactionEnum>, Error = String> + Send> {
        utxo_common::check_if_my_payment_sent(self.clone(), time_lock, other_pub, secret_hash)
    }

    fn search_for_swap_tx_spend_my(
        &self,
        time_lock: u32,
        other_pub: &[u8],
        secret_hash: &[u8],
        tx: &[u8],
        search_from_block: u64,
        _swap_contract_address: &Option<BytesJson>,
    ) -> Result<Option<FoundSwapTxSpend>, String> {
        utxo_common::search_for_swap_tx_spend_my(
            self.as_ref(),
            time_lock,
            other_pub,
            secret_hash,
            tx,
            utxo_common::DEFAULT_SWAP_VOUT,
            search_from_block,
        )
    }

    fn search_for_swap_tx_spend_other(
        &self,
        time_lock: u32,
        other_pub: &[u8],
        secret_hash: &[u8],
        tx: &[u8],
        search_from_block: u64,
        _swap_contract_address: &Option<BytesJson>,
    ) -> Result<Option<FoundSwapTxSpend>, String> {
        utxo_common::search_for_swap_tx_spend_other(
            self.as_ref(),
            time_lock,
            other_pub,
            secret_hash,
            tx,
            utxo_common::DEFAULT_SWAP_VOUT,
            search_from_block,
        )
    }

    fn extract_secret(&self, secret_hash: &[u8], spend_tx: &[u8]) -> Result<Vec<u8>, String> {
        utxo_common::extract_secret(secret_hash, spend_tx)
    }

    fn negotiate_swap_contract_addr(
        &self,
        _other_side_address: Option<&[u8]>,
    ) -> Result<Option<BytesJson>, MmError<NegotiateSwapContractAddrErr>> {
        Ok(None)
    }
}

impl MmCoin for ZCoin {
    fn is_asset_chain(&self) -> bool { self.utxo_arc.conf.asset_chain }

    fn withdraw(&self, _req: WithdrawRequest) -> WithdrawFut { todo!() }

    fn decimals(&self) -> u8 { self.utxo_arc.decimals }

    fn convert_to_address(&self, _from: &str, _to_address_format: Json) -> Result<String, String> { todo!() }

    fn validate_address(&self, _address: &str) -> ValidateAddressResult { todo!() }

    fn process_history_loop(&self, _ctx: MmArc) -> Box<dyn Future<Item = (), Error = ()> + Send> { todo!() }

    fn history_sync_status(&self) -> HistorySyncState { todo!() }

    fn get_trade_fee(&self) -> Box<dyn Future<Item = TradeFee, Error = String> + Send> {
        utxo_common::get_trade_fee(self.clone())
    }

    fn get_sender_trade_fee(&self, value: TradePreimageValue, stage: FeeApproxStage) -> TradePreimageFut<TradeFee> {
        utxo_common::get_sender_trade_fee(self.clone(), value, stage)
    }

    fn get_receiver_trade_fee(&self, _stage: FeeApproxStage) -> TradePreimageFut<TradeFee> {
        utxo_common::get_receiver_trade_fee(self.clone())
    }

    fn get_fee_to_send_taker_fee(
        &self,
        dex_fee_amount: BigDecimal,
        stage: FeeApproxStage,
    ) -> TradePreimageFut<TradeFee> {
        utxo_common::get_fee_to_send_taker_fee(self.clone(), dex_fee_amount, stage)
    }

    fn required_confirmations(&self) -> u64 { utxo_common::required_confirmations(&self.utxo_arc) }

    fn requires_notarization(&self) -> bool { utxo_common::requires_notarization(&self.utxo_arc) }

    fn set_required_confirmations(&self, confirmations: u64) {
        utxo_common::set_required_confirmations(&self.utxo_arc, confirmations)
    }

    fn set_requires_notarization(&self, requires_nota: bool) {
        utxo_common::set_requires_notarization(&self.utxo_arc, requires_nota)
    }

    fn swap_contract_address(&self) -> Option<BytesJson> { utxo_common::swap_contract_address() }

    fn mature_confirmations(&self) -> Option<u32> { Some(self.utxo_arc.conf.mature_confirmations) }
}

#[async_trait]
impl UtxoCommonOps for ZCoin {
    async fn get_tx_fee(&self) -> Result<ActualTxFee, JsonRpcError> { utxo_common::get_tx_fee(&self.utxo_arc).await }

    async fn get_htlc_spend_fee(&self, tx_size: u64) -> UtxoRpcResult<u64> {
        utxo_common::get_htlc_spend_fee(self, tx_size).await
    }

    fn addresses_from_script(&self, script: &Script) -> Result<Vec<Address>, String> {
        utxo_common::addresses_from_script(self.as_ref(), script)
    }

    fn denominate_satoshis(&self, satoshi: i64) -> f64 { utxo_common::denominate_satoshis(&self.utxo_arc, satoshi) }

    fn my_public_key(&self) -> &Public { self.utxo_arc.key_pair.public() }

    fn address_from_str(&self, address: &str) -> Result<Address, String> {
        utxo_common::checked_address_from_str(self.as_ref(), address)
    }

    async fn get_current_mtp(&self) -> UtxoRpcResult<u32> {
        utxo_common::get_current_mtp(&self.utxo_arc, CoinVariant::Standard).await
    }

    fn is_unspent_mature(&self, output: &RpcTransaction) -> bool {
        utxo_common::is_unspent_mature(self.utxo_arc.conf.mature_confirmations, output)
    }

    async fn calc_interest_if_required(
        &self,
        unsigned: TransactionInputSigner,
        data: AdditionalTxData,
        my_script_pub: Bytes,
    ) -> UtxoRpcResult<(TransactionInputSigner, AdditionalTxData)> {
        utxo_common::calc_interest_if_required(self, unsigned, data, my_script_pub).await
    }

    async fn calc_interest_of_tx(
        &self,
        _tx: &UtxoTx,
        _input_transactions: &mut HistoryUtxoTxMap,
    ) -> UtxoRpcResult<u64> {
        MmError::err(UtxoRpcError::Internal(
            "ZCoin doesn't support transaction rewards".to_owned(),
        ))
    }

    async fn get_mut_verbose_transaction_from_map_or_rpc<'a, 'b>(
        &'a self,
        tx_hash: H256Json,
        utxo_tx_map: &'b mut HistoryUtxoTxMap,
    ) -> UtxoRpcResult<&'b mut HistoryUtxoTx> {
        utxo_common::get_mut_verbose_transaction_from_map_or_rpc(self, tx_hash, utxo_tx_map).await
    }

    async fn p2sh_spending_tx(
        &self,
        prev_transaction: UtxoTx,
        redeem_script: Bytes,
        outputs: Vec<TransactionOutput>,
        script_data: Script,
        sequence: u32,
        lock_time: u32,
    ) -> Result<UtxoTx, String> {
        utxo_common::p2sh_spending_tx(
            self,
            prev_transaction,
            redeem_script,
            outputs,
            script_data,
            sequence,
            lock_time,
        )
        .await
    }

    async fn ordered_mature_unspents<'a>(
        &'a self,
        address: &Address,
    ) -> UtxoRpcResult<(Vec<UnspentInfo>, AsyncMutexGuard<'a, RecentlySpentOutPoints>)> {
        utxo_common::ordered_mature_unspents(self, address).await
    }

    fn get_verbose_transaction_from_cache_or_rpc(&self, txid: H256Json) -> UtxoRpcFut<VerboseTransactionFrom> {
        let selfi = self.clone();
        let fut = async move { utxo_common::get_verbose_transaction_from_cache_or_rpc(&selfi.utxo_arc, txid).await };
        Box::new(fut.boxed().compat())
    }

    async fn cache_transaction_if_possible(&self, tx: &RpcTransaction) -> Result<(), String> {
        utxo_common::cache_transaction_if_possible(&self.utxo_arc, tx).await
    }

    async fn list_unspent_ordered<'a>(
        &'a self,
        address: &Address,
    ) -> UtxoRpcResult<(Vec<UnspentInfo>, AsyncMutexGuard<'a, RecentlySpentOutPoints>)> {
        utxo_common::list_unspent_ordered(self, address).await
    }

    async fn preimage_trade_fee_required_to_send_outputs(
        &self,
        outputs: Vec<TransactionOutput>,
        fee_policy: FeePolicy,
        gas_fee: Option<u64>,
        stage: &FeeApproxStage,
    ) -> TradePreimageResult<BigDecimal> {
        utxo_common::preimage_trade_fee_required_to_send_outputs(self, outputs, fee_policy, gas_fee, stage).await
    }

    fn increase_dynamic_fee_by_stage(&self, dynamic_fee: u64, stage: &FeeApproxStage) -> u64 {
        utxo_common::increase_dynamic_fee_by_stage(self, dynamic_fee, stage)
    }

    async fn p2sh_tx_locktime(&self, htlc_locktime: u32) -> Result<u32, MmError<UtxoRpcError>> {
        utxo_common::p2sh_tx_locktime(self, self.ticker(), htlc_locktime).await
    }
}

#[test]
fn derive_z_key_from_mm_seed() {
    use common::privkey::key_pair_from_seed;
    use zcash_client_backend::encoding::encode_extended_spending_key;

    let seed = "spice describe gravity federal blast come thank unfair canal monkey style afraid";
    let secp_keypair = key_pair_from_seed(seed).unwrap();
    let z_spending_key = ExtendedSpendingKey::master(&*secp_keypair.private().secret);
    let encoded = encode_extended_spending_key(z_mainnet_constants::HRP_SAPLING_EXTENDED_SPENDING_KEY, &z_spending_key);
    assert_eq!(encoded, "secret-extended-key-main1qqqqqqqqqqqqqqytwz2zjt587n63kyz6jawmflttqu5rxavvqx3lzfs0tdr0w7g5tgntxzf5erd3jtvva5s52qx0ms598r89vrmv30r69zehxy2r3vesghtqd6dfwdtnauzuj8u8eeqfx7qpglzu6z54uzque6nzzgnejkgq569ax4lmk0v95rfhxzxlq3zrrj2z2kqylx2jp8g68lqu6alczdxd59lzp4hlfuj3jp54fp06xsaaay0uyass992g507tdd7psua5w6q76dyq3");

    let (_, address) = z_spending_key.default_address().unwrap();
    let encoded_addr = encode_payment_address(z_mainnet_constants::HRP_SAPLING_PAYMENT_ADDRESS, &address);
    assert_eq!(
        encoded_addr,
        "zs182ht30wnnnr8jjhj2j9v5dkx3qsknnr5r00jfwk2nczdtqy7w0v836kyy840kv2r8xle5gcl549"
    );

    let seed = "also shoot benefit prefer juice shell elder veteran woman mimic image kidney";
    let secp_keypair = key_pair_from_seed(seed).unwrap();
    let z_spending_key = ExtendedSpendingKey::master(&*secp_keypair.private().secret);
    let encoded = encode_extended_spending_key(z_mainnet_constants::HRP_SAPLING_EXTENDED_SPENDING_KEY, &z_spending_key);
    assert_eq!(encoded, "secret-extended-key-main1qqqqqqqqqqqqqq8jnhc9stsqwts6pu5ayzgy4szplvy03u227e50n3u8e6dwn5l0q5s3s8xfc03r5wmyh5s5dq536ufwn2k89ngdhnxy64sd989elwas6kr7ygztsdkw6k6xqyvhtu6e0dhm4mav8rus0fy8g0hgy9vt97cfjmus0m2m87p4qz5a00um7gwjwk494gul0uvt3gqyjujcclsqry72z57kr265jsajactgfn9m3vclqvx8fsdnwp4jwj57ffw560vvwks9g9hpu");

    let (_, address) = z_spending_key.default_address().unwrap();
    let encoded_addr = encode_payment_address(z_mainnet_constants::HRP_SAPLING_PAYMENT_ADDRESS, &address);
    assert_eq!(
        encoded_addr,
        "zs1funuwrjr2stlr6fnhkdh7fyz3p7n0p8rxase9jnezdhc286v5mhs6q3myw0phzvad5mvqgfxpam"
    );
}
