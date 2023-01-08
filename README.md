# poker

A poker implementation written in Rust.

The IO is implemented as a websocket server using the Actix framework.

To run the server locally, use ```cargo run```. Then open a browser to local host for the playable front end UI. 

You can also use ```cargo run -- --help``` for more settings.

Example:

```cargo run -- --ip 127.0.0.1 --port 8081```
