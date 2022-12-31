# poker

A poker implementation written in Rust.

The IO is implemented as a websocket server using the Actix framework.

To run the server locally, use ```cargo run --bin server```. Then open a browser to local host for the playable front end UI. 

You can also use ```cargo run --bin server -- --help``` for more settings.

Example:

```cargo run --bin server -- --ip 127.0.0.1 --port 8081```
