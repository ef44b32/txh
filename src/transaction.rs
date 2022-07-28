//! Types that model transactions, i.e. [`Deposit`]s and [`Withdrawal`]s.

use rust_decimal::Decimal;

use crate::ClientId;

/// Models a deposit.
#[derive(Clone, Debug)]
pub struct Deposit {
    pub client: ClientId,
    pub amount: Decimal,
    pub has_dispute: bool,
}

/// Models a withdrawal.
#[derive(Clone, Debug)]
pub struct Withdrawal {
    pub client: ClientId,
    pub amount: Decimal,
    pub has_dispute: bool,
}

/// The different types of transactions of the payment engine.
pub enum Transaction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
}

impl Transaction {
    /// Convenience function to create a [`Deposit`] variant.
    pub fn deposit(client: ClientId, amount: Decimal) -> Self {
        Self::Deposit(Deposit {
            client,
            amount,
            has_dispute: false,
        })
    }

    /// Convenience function to create a [`Withdrawal`] variant.
    pub fn withdrawal(client: ClientId, amount: Decimal) -> Self {
        Self::Withdrawal(Withdrawal {
            client,
            amount,
            has_dispute: false,
        })
    }
}
