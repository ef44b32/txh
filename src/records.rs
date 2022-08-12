//! Defines records that are used for serializing from and deserializing to CSV.

use rust_decimal::Decimal;
use thiserror::Error;

use crate::{event::Event, ClientId, TxId};

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid transaction type: `{0}`")]
    InvalidTransactionType(String),
}

/// Row format of an event in the input CSV file.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct EventCsvRecord {
    #[serde(rename = "type")]
    pub ty: String,
    pub client: ClientId,
    pub tx: TxId,
    pub amount: Decimal,
}

impl TryFrom<EventCsvRecord> for Event {
    type Error = Error;

    fn try_from(value: EventCsvRecord) -> Result<Self, Self::Error> {
        let EventCsvRecord { ty, client, tx, amount } = value;
        Ok(match ty.as_str() {
            "deposit" => Event::Deposit { client, tx, amount },
            "withdrawal" => Event::Withdrawal { client, tx, amount },
            "dispute" => Event::Dispute { client, tx },
            "resolve" => Event::Resolve { client, tx },
            "chargeback" => Event::Chargeback { client, tx },
            _ => Err(Error::InvalidTransactionType(ty))?,
        })
    }
}

/// Row format of a client in the output CSV file.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ClientCsvRecord {
    pub client: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

#[cfg(test)]
mod test {
    use rust_decimal_macros::dec;

    use super::*;

    impl EventCsvRecord {
        /// Helper function to create input rows for test.
        fn new(ty: &str, client: ClientId, tx: TxId, amount: Decimal) -> Self {
            Self {
                ty: ty.into(),
                client,
                tx,
                amount,
            }
        }
    }

    #[test]
    fn deserialize() -> Result<(), csv::Error> {
        let input = [
            "deposit,0,1,1234.5678",
            "withdrawal,0,1,-42.42",
            "dispute,0,1  ,0", // Whitespace
            "resolve,0,1,",    // Missing amount number
            "chargeback,0,1",  // Missing comma
        ];

        let expected = [
            EventCsvRecord::new("deposit", 0, 1, dec!(1234.5678)),
            EventCsvRecord::new("withdrawal", 0, 1, dec!(-42.42)),
            EventCsvRecord::new("dispute", 0, 1, dec!(0)),
            EventCsvRecord::new("resolve", 0, 1, dec!(0)),
            EventCsvRecord::new("chargeback", 0, 1, dec!(0)),
        ];

        // `.zip()` ends when the first iterator returns `None`, so we need to check for the correct length.
        assert_eq!(input.len(), expected.len());

        let input = input.join("\\n");

        let mut rdr = csv::Reader::from_reader(input.as_bytes());
        for (record, expected) in rdr.deserialize().zip(expected.into_iter()) {
            let record: EventCsvRecord = record?;
            assert_eq!(record, expected);
        }

        Ok(())
    }
}
