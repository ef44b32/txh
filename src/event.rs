//! Provides a richer structure of the CSV inputs for further processing.

use rust_decimal::Decimal;

use crate::{ClientId, TxId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Deposit {
        client: ClientId,
        tx: TxId,
        amount: Decimal,
    },
    Withdrawal {
        client: ClientId,
        tx: TxId,
        amount: Decimal,
    },
    Dispute {
        client: ClientId,
        tx: TxId,
    },
    Resolve {
        client: ClientId,
        tx: TxId,
    },
    Chargeback {
        client: ClientId,
        tx: TxId,
    },
}

#[cfg(test)]
// The following are convenience functions used for testing.
impl Event {
    pub fn deposit(client: ClientId, tx: TxId, amount: Decimal) -> Self {
        Event::Deposit { client, tx, amount }
    }

    pub fn withdrawal(client: ClientId, tx: TxId, amount: Decimal) -> Self {
        Event::Withdrawal { client, tx, amount }
    }

    pub fn dispute(client: ClientId, tx: TxId) -> Self {
        Event::Dispute { client, tx }
    }

    pub fn resolve(client: ClientId, tx: TxId) -> Self {
        Event::Resolve { client, tx }
    }

    pub fn chargeback(client: ClientId, tx: TxId) -> Self {
        Event::Chargeback { client, tx }
    }
}
