use crate::utxo::rpc_clients::UtxoRpcError;
use crate::NumConversError;
use bigdecimal::BigDecimal;
use derive_more::Display;
use rpc::v1::types::Bytes as BytesJson;
use rusqlite::Error as SqliteError;
use zcash_primitives::transaction::builder::Error as ZTxBuilderError;

#[derive(Debug, Display)]
pub enum GenTxError {
    DecryptedOutputNotFound,
    GetWitnessErr(GetUnspentWitnessErr),
    FailedToGetMerklePath,
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
    NumConversion(NumConversError),
    Rpc(UtxoRpcError),
    PrevTxNotConfirmed,
    TxBuilderError(ZTxBuilderError),
    #[display(fmt = "Failed to read ZCash tx from bytes {:?} with error {}", hex, err)]
    TxReadError {
        hex: BytesJson,
        err: std::io::Error,
    },
}

impl From<GetUnspentWitnessErr> for GenTxError {
    fn from(err: GetUnspentWitnessErr) -> GenTxError { GenTxError::GetWitnessErr(err) }
}

impl From<NumConversError> for GenTxError {
    fn from(err: NumConversError) -> GenTxError { GenTxError::NumConversion(err) }
}

impl From<UtxoRpcError> for GenTxError {
    fn from(err: UtxoRpcError) -> GenTxError { GenTxError::Rpc(err) }
}

impl From<ZTxBuilderError> for GenTxError {
    fn from(err: ZTxBuilderError) -> GenTxError { GenTxError::TxBuilderError(err) }
}

#[derive(Debug, Display)]
#[allow(clippy::large_enum_variant)]
pub enum SendOutputsErr {
    GenTxError(GenTxError),
    NumConversion(NumConversError),
    Rpc(UtxoRpcError),
    TxNotMined(String),
}

impl From<GenTxError> for SendOutputsErr {
    fn from(err: GenTxError) -> SendOutputsErr { SendOutputsErr::GenTxError(err) }
}

impl From<NumConversError> for SendOutputsErr {
    fn from(err: NumConversError) -> SendOutputsErr { SendOutputsErr::NumConversion(err) }
}

impl From<UtxoRpcError> for SendOutputsErr {
    fn from(err: UtxoRpcError) -> SendOutputsErr { SendOutputsErr::Rpc(err) }
}

#[derive(Debug, Display)]
pub enum GetUnspentWitnessErr {
    EmptyDbResult,
    TreeOrWitnessAppendFailed,
    OutputCmuNotFoundInCache,
    Sql(SqliteError),
}

impl From<SqliteError> for GetUnspentWitnessErr {
    fn from(err: SqliteError) -> GetUnspentWitnessErr { GetUnspentWitnessErr::Sql(err) }
}

#[derive(Debug, Display)]
pub enum ZCoinBuildError {
    BuilderError(String),
    GetAddressError,
    SqliteError(SqliteError),
}

impl From<SqliteError> for ZCoinBuildError {
    fn from(err: SqliteError) -> ZCoinBuildError { ZCoinBuildError::SqliteError(err) }
}
