//! The main business logic of our application.

use std::collections::HashMap;

use thiserror::Error;

use crate::{
    client::{ClientState, Transition},
    event::Event,
    transaction::{Deposit, Transaction, Withdrawal},
    ClientId, TxId,
};

/// Errors that can happen during processing.
#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("duplicate transaction id: `{0}`")]
    DuplicateTxId(TxId),
}

/// Stores all the information that is required to compute the client state.
///
/// Note that this implmentation is not safe to be used in a concurrent environment.
pub struct State {
    // This duplicates the `TxId` because it is also contained in `Transfer`.
    transfers: HashMap<TxId, Transaction>,
    client_states: HashMap<ClientId, ClientState>,
}

impl State {
    pub fn new() -> Self {
        Self {
            transfers: HashMap::new(),
            client_states: HashMap::new(),
        }
    }

    pub fn handle(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::Deposit { client, amount, tx } => {
                let state = self.client_states.entry(client).or_default();

                if let Ok(next_state) = state.clone().apply(Transition::Deposit(amount)) {
                    *state = next_state;

                    match self.transfers.insert(tx, Transaction::deposit(client, amount)) {
                        Some(_) => Err(Error::DuplicateTxId(tx)),
                        None => Ok(()),
                    }?;
                }
            }
            Event::Withdrawal { client, amount, tx } => {
                let state = self.client_states.entry(client).or_default();
                if let Ok(next_state) = state.clone().apply(Transition::Withdrawal(amount)) {
                    *state = next_state;

                    match self.transfers.insert(tx, Transaction::withdrawal(client, amount)) {
                        Some(_) => Err(Error::DuplicateTxId(tx)),
                        None => Ok(()),
                    }?;
                }
            }
            Event::Chargeback { client, tx } => {
                // Assumption: Chargebacks only make sense for Deposits
                if let Some(Transaction::Deposit(deposit)) = self.transfers.get(&tx) {
                    // Skip processing if the transfer and chargeback client don't match.
                    if client != deposit.client {
                        return Ok(());
                    }

                    if let Some(state) = self.client_states.get_mut(&client) {
                        if let Ok(next_state) = state.clone().apply(Transition::Chargeback) {
                            *state = next_state;
                        }
                    }
                }
            }
            Event::Dispute { client, tx } => {
                if let Some(Transaction::Deposit(deposit)) = self.transfers.get_mut(&tx) {
                    // Skip processing if the transfer and chargeback client don't match.
                    if client != deposit.client || deposit.has_dispute {
                        return Ok(());
                    }

                    if let Some(state) = self.client_states.get_mut(&client) {
                        if let Ok(next_state) = state.clone().apply(Transition::DisputeDeposit(deposit.amount)) {
                            *state = next_state;
                            deposit.has_dispute = true;
                        }
                    }
                } else if let Some(Transaction::Withdrawal(withdrawal)) = self.transfers.get_mut(&tx) {
                    // Skip processing if the transfer and chargeback client don't match.
                    if client != withdrawal.client || withdrawal.has_dispute {
                        return Ok(());
                    }

                    if let Some(state) = self.client_states.get_mut(&client) {
                        if let Ok(next_state) = state.clone().apply(Transition::DisputeWithdrawal(withdrawal.amount)) {
                            *state = next_state;
                            withdrawal.has_dispute = true;
                        }
                    }
                }
            }
            Event::Resolve {
                client: resolve_client,
                tx,
            } => {
                match self.transfers.get_mut(&tx) {
                    Some(
                        Transaction::Deposit(Deposit {
                            client,
                            has_dispute,
                            amount,
                        })
                        | Transaction::Withdrawal(Withdrawal {
                            client,
                            has_dispute,
                            amount,
                        }),
                    ) => {
                        // Skip processing if the tx and chargeback client don't match or if there is no active dispute.
                        if resolve_client != *client || !*has_dispute {
                            return Ok(());
                        }

                        if let Some(state) = self.client_states.get_mut(&client) {
                            if let Ok(next_state) = state.clone().apply(Transition::Resolve(*amount)) {
                                *state = next_state;
                                *has_dispute = false;
                            }
                        }
                    }
                    None => (),
                }
            }
        }
        Ok(())
    }

    pub fn client_states(&self) -> impl Iterator<Item = (&ClientId, &ClientState)> {
        self.client_states.iter()
    }
}

#[cfg(test)]
mod test {
    use rust_decimal_macros::dec;

    use super::*;

    impl State {
        fn handle_multiple(&mut self, stream: impl IntoIterator<Item = Event>) -> Result<(), Error> {
            for e in stream {
                self.handle(e)?;
            }
            Ok(())
        }
    }

    #[test]
    fn deposit_and_withdrawal() -> Result<(), Error> {
        let mut state = State::new();

        #[rustfmt::skip]
        state.handle_multiple([
            Event::deposit(0, 0, dec!(17)),
            Event::withdrawal(0, 1, dec!(9)),
        ])?;

        let expected: HashMap<ClientId, ClientState> =
            [(0, ClientState::new(false, dec!(8), dec!(0)))].into_iter().collect();

        assert_eq!(state.client_states, expected);

        Ok(())
    }

    #[test]
    fn invalid_withdrawal() -> Result<(), Error> {
        let mut state = State::new();

        #[rustfmt::skip]
        state.handle_multiple([
            Event::deposit(0, 0, dec!(17)),
            Event::withdrawal(0, 1, dec!(18)),
        ])?;

        let expected = HashMap::from([(0, ClientState::new(false, dec!(17), dec!(0)))]);

        assert_eq!(state.client_states, expected);

        Ok(())
    }

    #[test]
    fn chargeback() -> Result<(), Error> {
        let mut state = State::new();

        let (client_a, tx_a) = (11, 0);
        let (client_b, tx_b) = (12, 10);

        state.handle_multiple([
            Event::deposit(client_a, tx_a, dec!(11)),
            Event::deposit(client_b, tx_b, dec!(12)),
            Event::chargeback(client_a, tx_b), // can't chargeback tx that does not belong to client
            Event::chargeback(client_b, tx_a), // can't chargeback tx that does not belong to client
            Event::withdrawal(client_a, tx_a + 1, dec!(10)),
            Event::chargeback(client_a, tx_a + 1), // can't chargeback withdrawals
        ])?;

        let expected = HashMap::from([
            (client_a, ClientState::new(false, dec!(1), dec!(0))),
            (client_b, ClientState::new(false, dec!(12), dec!(0))),
        ]);
        assert_eq!(state.client_states, expected);

        state.handle_multiple([
            Event::chargeback(client_a, tx_a),              // freezing `client_a`
            Event::deposit(client_a, tx_a + 2, dec!(2)),    // no effect
            Event::withdrawal(client_a, tx_a + 3, dec!(2)), // no effect
            Event::dispute(client_a, tx_a),                 // no effect
        ])?;

        let expected = HashMap::from([
            (client_a, ClientState::new(true, dec!(1), dec!(0))),
            (client_b, ClientState::new(false, dec!(12), dec!(0))),
        ]);
        assert_eq!(state.client_states, expected);

        Ok(())
    }

    #[test]
    fn dispute_resolve() -> Result<(), Error> {
        let mut state = State::new();

        state.handle_multiple([
            Event::deposit(0, 0, dec!(17)),
            Event::deposit(0, 1, dec!(42)),
            Event::dispute(0, 0),
        ])?;

        let expected = HashMap::from([(0, ClientState::new(false, dec!(42), dec!(17)))]);
        assert_eq!(state.client_states, expected);

        state.handle_multiple([
            Event::withdrawal(0, 2, dec!(43)), // insufficient available funds
            Event::resolve(1, 0),              // can't resolve a transaction if client ids don't match
            Event::resolve(0, 1),              // can't resolve a transaction without dispute
            Event::dispute(0, 0),              // disputing twice has no effect
            Event::resolve(0, 0),
        ])?;

        let expected = HashMap::from([(0, ClientState::new(false, dec!(59), dec!(0)))]);
        assert_eq!(state.client_states, expected);

        state.handle_multiple([
            Event::withdrawal(0, 2, dec!(43)), // this works now available funds (note the id can be reused)
            Event::resolve(0, 0),              // can't resolve a transaction twice
            Event::dispute(0, 2),              // dispute a withdrawal
        ])?;

        let expected = HashMap::from([(0, ClientState::new(false, dec!(16), dec!(43)))]);
        assert_eq!(state.client_states, expected);

        state.handle_multiple([
            Event::resolve(0, 2), // resolve the withdrawal
        ])?;

        let expected = HashMap::from([(0, ClientState::new(false, dec!(59), dec!(0)))]);
        assert_eq!(state.client_states, expected);

        Ok(())
    }

    #[test]
    fn dispute_insufficient_funds() -> Result<(), Error> {
        let mut state = State::new();

        state.handle_multiple([
            Event::deposit(0, 0, dec!(42)),
            Event::withdrawal(0, 1, dec!(41)),
            Event::dispute(0, 0), // can't dispute if to much funds have been withdrawn
        ])?;

        let expected = HashMap::from([(0, ClientState::new(false, dec!(1), dec!(0)))]);
        assert_eq!(state.client_states, expected);

        Ok(())
    }

    #[test]
    fn only_invalid() -> Result<(), Error> {
        let mut state = State::new();

        // We don't want to create clients in our storage if all their transactions are invalid.
        state.handle_multiple([Event::resolve(0, 0), Event::dispute(0, 1), Event::chargeback(0, 2)])?;

        let expected = HashMap::new();
        assert_eq!(state.client_states, expected);

        Ok(())
    }
}
