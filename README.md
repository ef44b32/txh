# txh

The documentation of this project is written as doc comments.
It can be viewed by running the following command:

```sh
cargo doc --open --document-private-items
```

## About

There is a simple CSV file in `example/data/simple.csv` but most of the testing
happens in unit tests in the individual modules. To run the problem simply type:

```sh
cargo run -- examples/data/simple.csv
```

### Features

* No `.unwrap()` in own code
* No `unsafe` code
* Updating the client is done using a simple state machine
* Proper error types and reporting using the `anyhow` crate.

### Limitations

The current implementation will struggle to load very large datasets where the
number of transactions is close to `u32::MAX`. This is because the entire state
is stored in memory without persistance. The key data structures for the overall
state are `HashMap` so a key-value store could be used to use the disk in those
cases.

### Assumptions

The implementation makes the following assumptions:

* The dataset fits into memory.
* A deposit can only be disputed, if the user still has enough funds.
* Withdrawals can be disputed and if resolved, their value is transfered back to
  the user.
* Chargebacks can only occur on deposits.
