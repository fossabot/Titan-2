# Titan

![status](https://img.shields.io/travis/com/r-spacex/Titan)
![license](https://img.shields.io/github/license/r-spacex/Titan)

[Documentation](https://r-spacex.github.io/Titan)

## Build process

In order to build the API,
you must have Rust installed.
If you do not already,
you can find the instructions on how to do so
[in the Rust book](https://doc.rust-lang.org/1.0.0/book/installing-rust.html).

To clone this repository,
run `git clone git@github.com:r-spacex/Titan.git`.

Once inside the repository,
you can build a binary for development with `cargo build`.
For release (which has optimizations enabled),
run `cargo build --release`.
The resulting binaries will be `./target/debug/titan` and `./target/release/titan` respectively.

Please note that the release build is identical to the debug build,
with the only differences being in performance
and some endpoints being disabled in release.
As such, it is highly recommended to only build with the `--release` flag when necessary.

### CLI options

Run `./titan --help` to view the CLI options.

Currently, you can set the socket for the REST server via `-r` or `--rest-server`,
followed by an IP _and_ the port.
Likewise, you can set the socket for the WebSocket server via `-w` or `--ws-server`.

Telemetry logging is opt-in when starting the server.
To do so, pass `-t` or `--telemetry`.

## Database changes

If you're making a change to the database itself,
you'll likely want the Diesel CLI installed (`cargo install diesel`).
Look up Diesel's CLI syntax if you're not familiar.
The only thing of note is that, for this project,
it is preferred to use sequential IDs rather than the datetime.

After creating a migration, be sure to run `diesel migrate run`!
Without that, `api/src/schema.rs` will not be updated,
and you won't be able to compile any changes relying on it.

## Testing

Tests on each endpoint are located in the `api/src/tests` directory.
As of present, unit tests have not been created,
and may not in the future.

To run all tests, run `cargo test`.

## Commits

Before commiting,
be sure to run `./precommit`.
This is the _exact_ script run as a test after you push,
so doing this will ensure tests pass.
