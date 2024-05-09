use pathfinder_common::event::Event;
use pathfinder_common::receipt::Receipt;
use pathfinder_common::transaction::{Transaction, TransactionVariant};
use pathfinder_common::{BlockHash, BlockNumber, TransactionHash, TransactionVersion};

use super::serialize;
use crate::dto::serialize::{SerializeForVersion, Serializer};
use crate::{dto, RpcVersion};

#[derive(Copy, Clone)]
pub enum TxnStatus {
    Received,
    Rejected,
    AcceptedOnL2,
    AcceptedOnL1,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TxnExecutionStatus {
    Succeeded,
    Reverted,
}

impl From<&pathfinder_common::receipt::ExecutionStatus> for TxnExecutionStatus {
    fn from(value: &pathfinder_common::receipt::ExecutionStatus) -> Self {
        use pathfinder_common::receipt::ExecutionStatus;
        match value {
            ExecutionStatus::Succeeded => Self::Succeeded,
            ExecutionStatus::Reverted { .. } => Self::Reverted,
        }
    }
}

#[derive(Copy, Clone)]
pub enum TxnFinalityStatus {
    AcceptedOnL2,
    AcceptedOnL1,
}

pub struct TxnReceiptWithBlockInfo<'a> {
    pub block_hash: Option<&'a BlockHash>,
    pub block_number: Option<BlockNumber>,
    pub receipt: &'a Receipt,
    pub transaction: &'a Transaction,
    pub events: &'a [Event],
    pub finality: TxnFinalityStatus,
}

pub struct TxnReceipt<'a> {
    pub receipt: &'a Receipt,
    pub transaction: &'a Transaction,
    pub events: &'a [Event],
    pub finality: TxnFinalityStatus,
}

pub struct CommonReceiptProperties<'a> {
    pub receipt: &'a Receipt,
    pub transaction: &'a Transaction,
    pub events: &'a [Event],
    pub finality: TxnFinalityStatus,
}

#[derive(Copy, Clone)]
pub struct PriceUnit<'a>(pub &'a TransactionVersion);

pub struct FeePayment<'a> {
    amount: &'a pathfinder_common::Fee,
    transaction_version: &'a TransactionVersion,
}
pub struct MsgToL1<'a>(pub &'a pathfinder_common::receipt::L2ToL1Message);
pub struct ExecutionResources<'a>(pub &'a pathfinder_common::receipt::ExecutionResources);

impl SerializeForVersion for TxnStatus {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        match self {
            TxnStatus::Received => "RECEIVED",
            TxnStatus::Rejected => "REJECTED",
            TxnStatus::AcceptedOnL2 => "ACCEPTED_ON_L2",
            TxnStatus::AcceptedOnL1 => "ACCEPTED_ON_L1",
        }
        .serialize(serializer)
    }
}

impl SerializeForVersion for TxnExecutionStatus {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        match self {
            TxnExecutionStatus::Succeeded => "SUCCEEDED",
            TxnExecutionStatus::Reverted => "REVERTED",
        }
        .serialize(serializer)
    }
}

impl SerializeForVersion for TxnFinalityStatus {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        match self {
            TxnFinalityStatus::AcceptedOnL2 => "ACCEPTED_ON_L2",
            TxnFinalityStatus::AcceptedOnL1 => "ACCEPTED_ON_L1",
        }
        .serialize(serializer)
    }
}

impl SerializeForVersion for TxnReceiptWithBlockInfo<'_> {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        let Self {
            block_hash,
            block_number,
            receipt,
            transaction,
            events,
            finality,
        } = self;

        let mut serializer = serializer.serialize_struct()?;

        serializer.flatten(&TxnReceipt {
            receipt,
            transaction,
            events,
            finality: *finality,
        })?;

        serializer.serialize_optional("block_hash", block_hash.map(dto::BlockHash))?;
        serializer.serialize_optional("block_number", block_number.map(dto::BlockNumber))?;

        serializer.end()
    }
}

impl SerializeForVersion for TxnReceipt<'_> {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        todo!()
    }
}

impl SerializeForVersion for CommonReceiptProperties<'_> {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        let mut serializer = serializer.serialize_struct()?;

        serializer.serialize_field("transaction_hash", &dto::TxnHash(&self.transaction.hash))?;
        serializer.serialize_field(
            "actual_fee",
            &FeePayment {
                amount: &self.receipt.actual_fee,
                transaction_version: &self.transaction.version(),
            },
        )?;
        serializer.serialize_field("finality_status", &self.finality)?;
        serializer.serialize_iter(
            "messages_sent",
            self.receipt.l2_to_l1_messages.len(),
            &mut self.receipt.l2_to_l1_messages.iter().map(MsgToL1),
        )?;
        serializer.serialize_iter(
            "events",
            self.events.len(),
            &mut self.events.iter().map(|e| dto::Event {
                address: &e.from_address,
                keys: &e.keys,
                data: &e.data,
            }),
        )?;
        serializer.serialize_field(
            "execution_resources",
            &ExecutionResources(&self.receipt.execution_resources),
        )?;

        serializer.end()
    }
}

impl SerializeForVersion for FeePayment<'_> {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        let mut serializer = serializer.serialize_struct()?;

        serializer.serialize_field("amount", &dto::Felt(&self.amount.0))?;
        serializer.serialize_field("unit", &PriceUnit(&self.transaction_version))?;

        serializer.end()
    }
}

impl SerializeForVersion for MsgToL1<'_> {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        todo!()
    }
}

impl SerializeForVersion for ExecutionResources<'_> {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        todo!()
    }
}

impl SerializeForVersion for PriceUnit<'_> {
    fn serialize(&self, serializer: Serializer) -> Result<serialize::Ok, serialize::Error> {
        match self.0 {
            &TransactionVersion::ZERO | &TransactionVersion::ONE | &TransactionVersion::TWO => {
                "WEI"
            }
            _ => "FRI",
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions_sorted::assert_eq;
    use rstest::rstest;
    use serde_json::json;

    use super::*;
    use crate::dto::serialize::Serializer;

    #[rstest]
    #[case::received(TxnStatus::Received, "RECEIVED")]
    #[case::rejected(TxnStatus::Rejected, "REJECTED")]
    #[case::accepted_on_l2(TxnStatus::AcceptedOnL2, "ACCEPTED_ON_L2")]
    #[case::accepted_on_l1(TxnStatus::AcceptedOnL1, "ACCEPTED_ON_L1")]
    fn txn_status(#[case] input: TxnStatus, #[case] expected: &str) {
        let expected = json!(expected);
        let encoded = input.serialize(Serializer::default()).unwrap();
        assert_eq!(encoded, expected);
    }

    #[rstest]
    #[case::accepted_on_l2(TxnFinalityStatus::AcceptedOnL2, "ACCEPTED_ON_L2")]
    #[case::accepted_on_l1(TxnFinalityStatus::AcceptedOnL1, "ACCEPTED_ON_L1")]
    fn txn_finality_status(#[case] input: TxnFinalityStatus, #[case] expected: &str) {
        let expected = json!(expected);
        let encoded = input.serialize(Serializer::default()).unwrap();
        assert_eq!(encoded, expected);
    }

    #[rstest]
    #[case::accepted_on_l2(TxnExecutionStatus::Succeeded, "SUCCEEDED")]
    #[case::accepted_on_l1(TxnExecutionStatus::Reverted, "REVERTED")]
    fn txn_execution_status(#[case] input: TxnExecutionStatus, #[case] expected: &str) {
        let expected = json!(expected);
        let encoded = input.serialize(Serializer::default()).unwrap();
        assert_eq!(encoded, expected);
    }
}
