# poker
a playable poker implementation in Rust.

currently the game logic is complete, and there is a super dumb (random) AI to play against in the terminal.

The is a websocket-based server and client implemented using the actix framework.

to run the server locally, use ```cargo run --bin server```, and in a separate tab, use ```cargo run --bin client``` to connect.

**Lobby commands are:**
```
/name X [change your name]
/join X [join (and create if need be) table X]
```

Once you create a table, currently two bots will be added, and it will start playing hands indefinitely.

**In-game commands are:**
```
/check
/call
/fold
/bet X
/leave [brings you back to the lobby (where you can join a new table)
```
