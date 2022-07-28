//! A simple state machine to capture changes in user funds.

use rust_decimal::Decimal;
use thiserror::Error;

/// [`ClientState`] is a simple state machine that captures the state of the funds of a user.
///
/// The fields are private so that they can only be changed through transitions in the state machine, which should make
/// our implementation more robust and easier to reason about.
#[must_use]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientState {
    frozen: bool,
    available: Decimal,
    held: Decimal,
    // We can always compute the total from `available` and `held`.
}

/// Errors that can happen during state transitions.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("client is frozen")]
    ClientFrozen,
    #[error("insufficient funds")]
    InsufficientFunds,
}

/// The different transitions of the state machine.
#[derive(Clone, Copy, Debug)]
pub enum Transition {
    Deposit(Decimal),
    Withdrawal(Decimal),
    DisputeDeposit(Decimal),
    DisputeWithdrawal(Decimal),
    Resolve(Decimal),
    Chargeback,
}

impl ClientState {
    /// Returns `true` if the client account is frozen.
    pub fn frozen(&self) -> bool {
        self.frozen
    }

    /// Returns funds available to the client.
    pub fn available(&self) -> Decimal {
        self.available
    }

    /// Returns the client's funds that are currently held due to a dispute.
    pub fn held(&self) -> Decimal {
        self.held
    }

    /// Returns the total funds of a client, which is the sum of [`Self::available()`] and [`Self::held()`].
    pub fn total(&self) -> Decimal {
        self.available() + self.held()
    }

    /// Changes the state of funds by applying a transaction.
    ///
    /// This is the state transition in our state machine.
    pub fn apply(mut self, transition: Transition) -> Result<Self, Error> {
        use Transition::*;
        match (transition, &mut self) {
            (_, ClientState { frozen: true, .. }) => return Err(Error::ClientFrozen),
            (Chargeback, ClientState { frozen, .. }) => *frozen = true,
            (Deposit(amount), ClientState { available, .. }) => *available += amount,
            (Withdrawal(amount), ClientState { available, .. }) => match *available < amount {
                true => return Err(Error::InsufficientFunds),
                false => *available -= amount,
            },
            (DisputeDeposit(amount), ClientState { available, held, .. }) => match *available < amount {
                true => return Err(Error::InsufficientFunds),
                false => {
                    *available -= amount;
                    *held += amount;
                }
            },
            (DisputeWithdrawal(amount), ClientState { held, .. }) => *held += amount,
            (Resolve(amount), ClientState { available, held, .. }) => {
                *available += amount;
                *held -= amount;
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use rust_decimal_macros::dec;

    use super::{
        Transition::{Chargeback, Deposit, DisputeDeposit, Resolve, Withdrawal},
        *,
    };

    // Used by other modules for testing.
    impl ClientState {
        pub fn new(frozen: bool, available: Decimal, held: Decimal) -> Self {
            Self {
                frozen,
                available,
                held,
            }
        }
    }

    #[test]
    fn deposit() -> Result<(), Error> {
        let state = ClientState::default()
            .apply(Deposit(dec!(42)))?
            .apply(Deposit(dec!(58)))?
            .apply(Deposit(dec!(1)))?;
        assert_eq!(state.available, dec!(101));

        Ok(())
    }

    #[test]
    fn withdrawal() -> Result<(), Error> {
        let state = ClientState::default().apply(Withdrawal(dec!(1)));
        assert_eq!(state, Err(Error::InsufficientFunds));
        let state = ClientState::default()
            .apply(Deposit(dec!(100)))?
            .apply(Withdrawal(dec!(99)))?;
        assert_eq!(state.available, dec!(1));

        Ok(())
    }

    #[test]
    fn frozen() -> Result<(), Error> {
        let state = ClientState::default().apply(Deposit(dec!(42)))?.apply(Chargeback)?;
        assert_eq!(state.available, dec!(42));
        assert_eq!(state.frozen, true);
        let state = state.apply(Deposit(dec!(42)));
        assert_eq!(state, Err(Error::ClientFrozen));

        Ok(())
    }

    #[test]
    fn dispute_resolve() -> Result<(), Error> {
        let state = ClientState::default().apply(Withdrawal(dec!(1)));
        assert_eq!(state, Err(Error::InsufficientFunds));
        let state = ClientState::default()
            .apply(Deposit(dec!(100)))?
            .apply(DisputeDeposit(dec!(99)))?;
        assert_eq!(state.available, dec!(1));
        assert_eq!(state.held, dec!(99));
        let state = state.apply(Resolve(dec!(99)))?;
        assert_eq!(state.available, dec!(100));
        assert_eq!(state.held, dec!(0));

        Ok(())
    }
}
