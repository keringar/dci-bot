# DCI Bot

This is a scraper that takes in the DCI event listings and times and
stores it in a sqlite3 database.

## Dependencies

`libssl` and `sqlite3`

## Building

1. Install rust and libsqlite3-dev
    `curl https://sh.rustup.rs -sSf | sh`
    `sudo apt update && sudo apt install libsqlite3-dev`

2. Clone this repository
    `git clone https://github.com/keringar/dci-bot`

3. Build
    `cd dci-bot`
    `cargo build --release`

4. Copy the output file in dci-bot/target/release/dci-bot to your desired working directory

## Running

Set the required environment variables `DCI_PASSWORD` and `DCI_SECRET` where password is the account
password and secret is the script secret.
