// Usually it is better to catch these things in a CI workflow, but for this small project the following is enough.
#![warn(missing_docs)]
#![warn(warnings)]

//! This tool can be used to retrieve the client status from a list of transactions in the form of a CSV file.

mod client;
mod event;
mod records;
mod state;
mod transaction;

use std::{env, fs::File, io};

use anyhow::{Context as _, Result};
use csv::WriterBuilder;
use records::ClientCsvRecord;
use state::State;

use self::{event::Event, records::EventCsvRecord};

/// Uniquely refers to a client.
type ClientId = u16;
/// Uniquely refers to a transaction.
type TxId = u32;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: txh <input_file>.csv");
        return Ok(());
    }
    let filename = &args[1];
    let file = File::open(filename).context(format!("Failed to read CSV from `{filename}`."))?;

    let mut state = State::new();

    // Read from CSV file
    let mut rdr = csv::ReaderBuilder::new().has_headers(false).from_reader(file);
    for record in rdr.deserialize() {
        let record: EventCsvRecord = record?;
        let event = Event::try_from(record)?;

        state.handle(event)?;
    }

    // Output to stdout
    let mut wtr = WriterBuilder::new().has_headers(true).from_writer(io::stdout());
    for (&client, state) in state.client_states() {
        let record = ClientCsvRecord {
            client,
            available: state.available(),
            held: state.held(),
            total: state.total(),
            locked: state.frozen(),
        };

        wtr.serialize(record)?;
    }

    Ok(())
}
