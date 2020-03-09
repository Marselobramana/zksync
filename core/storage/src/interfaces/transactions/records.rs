// External imports
use chrono::prelude::*;
use diesel::sql_types::{BigInt, Bool, Int4, Jsonb, Nullable, Text};
use serde_derive::{Deserialize, Serialize};
use serde_json::value::Value;
// Workspace imports
use models::node::block::ExecutedTx;
use models::node::{BlockNumber, FranklinOp, FranklinTx};
// Local imports
use crate::interfaces::prover::records::ProverRun;
use crate::schema::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountTransaction {
    pub tx: Value,
    pub tx_hash: String,
    pub success: bool,
    pub fail_reason: Option<String>,
    pub committed: bool,
    pub verified: bool,
}

#[derive(Debug, Insertable)]
#[table_name = "mempool"]
pub struct InsertTx {
    pub hash: Vec<u8>,
    pub primary_account_address: Vec<u8>,
    pub nonce: i64,
    pub tx: Value,
}

#[derive(Debug, Queryable)]
pub struct ReadTx {
    pub hash: Vec<u8>,
    pub primary_account_address: Vec<u8>,
    pub nonce: i64,
    pub tx: Value,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
pub struct TransactionsHistoryItem {
    #[sql_type = "Nullable<Text>"]
    pub hash: Option<String>,

    #[sql_type = "Nullable<BigInt>"]
    pub pq_id: Option<i64>,

    #[sql_type = "Jsonb"]
    pub tx: Value,

    #[sql_type = "Nullable<Bool>"]
    pub success: Option<bool>,

    #[sql_type = "Nullable<Text>"]
    pub fail_reason: Option<String>,

    #[sql_type = "Bool"]
    pub commited: bool,

    #[sql_type = "Bool"]
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReceiptResponse {
    pub tx_hash: String,
    pub block_number: i64,
    pub success: bool,
    pub verified: bool,
    pub fail_reason: Option<String>,
    pub prover_run: Option<ProverRun>,
}

// TODO: jazzandrock add more info(?)
#[derive(Debug, Serialize, Deserialize)]
pub struct PriorityOpReceiptResponse {
    pub committed: bool,
    pub verified: bool,
    pub prover_run: Option<ProverRun>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, QueryableByName)]
pub struct TxByHashResponse {
    #[sql_type = "Text"]
    pub tx_type: String, // all

    #[sql_type = "Text"]
    pub from: String, // transfer(from) | deposit(our contract) | withdraw(sender)

    #[sql_type = "Text"]
    pub to: String, // transfer(to) | deposit(sender) | withdraw(our contract)

    #[sql_type = "Int4"]
    pub token: i32, // all

    #[sql_type = "Text"]
    pub amount: String, // all

    #[sql_type = "Nullable<Text>"]
    pub fee: Option<String>, // means Sync fee, not eth. transfer(sync fee), deposit(none), withdraw(Sync fee)

    #[sql_type = "BigInt"]
    pub block_number: i64, // all
}

#[derive(Debug, Queryable, QueryableByName)]
#[table_name = "executed_transactions"]
pub struct StoredExecutedTransaction {
    pub id: i32,
    pub block_number: i64,
    pub tx_hash: Vec<u8>,
    pub operation: Option<Value>,
    pub success: bool,
    pub fail_reason: Option<String>,
    pub block_index: Option<i32>,
}

impl StoredExecutedTransaction {
    pub fn into_executed_tx(
        self,
        stored_tx: Option<super::ReadTx>,
    ) -> Result<ExecutedTx, failure::Error> {
        if let Some(op) = self.operation {
            let franklin_op: FranklinOp =
                serde_json::from_value(op).expect("Unparsable FranklinOp in db");
            Ok(ExecutedTx {
                tx: franklin_op
                    .try_get_tx()
                    .expect("FranklinOp should not have tx"),
                success: true,
                op: Some(franklin_op),
                fail_reason: None,
                block_index: Some(self.block_index.expect("Block idx should be set") as u32),
            })
        } else if let Some(stored_tx) = stored_tx {
            let tx: FranklinTx = serde_json::from_value(stored_tx.tx).expect("Unparsable tx in db");
            Ok(ExecutedTx {
                tx,
                success: false,
                op: None,
                fail_reason: self.fail_reason,
                block_index: None,
            })
        } else {
            failure::bail!("Unsuccessful tx was lost from db.");
        }
    }
}

#[derive(Debug, Insertable)]
#[table_name = "executed_transactions"]
pub struct NewExecutedTransaction {
    pub block_number: i64,
    pub tx_hash: Vec<u8>,
    pub operation: Option<Value>,
    pub success: bool,
    pub fail_reason: Option<String>,
    pub block_index: Option<i32>,
}

impl NewExecutedTransaction {
    pub fn prepare_stored_tx(exec_tx: &ExecutedTx, block: BlockNumber) -> Self {
        Self {
            block_number: i64::from(block),
            tx_hash: exec_tx.tx.hash().as_ref().to_vec(),
            operation: exec_tx.op.clone().map(|o| serde_json::to_value(o).unwrap()),
            success: exec_tx.success,
            fail_reason: exec_tx.fail_reason.clone(),
            block_index: exec_tx.block_index.map(|idx| idx as i32),
        }
    }
}
