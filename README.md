# poker

A poker implementation written in Rust.

The IO is implemented as a websocket server using the Actix framework.

To run the server locally, use ```cargo run```. Then open a browser to local host for the playable front end UI. 

You can also use ```cargo run -- --help``` for more settings.

Example:

```
cargo run -- --ip 127.0.0.1 --port 8080
```

You can also run it inside a docker container via:

```
docker build --tag poker --file Dockerfile .
```

and then

```
docker run -p 8080:8080 poker
```

Note: in this case, you must go to your actual ip address in the browser, since localhost won't work with the docker container.


## React UI

The UI is handled by React.

### Setup

First make sure you have Node.js and npm installed.

```
cd react-ui
npm install
```

### Run Locally

For live development you can run the server and React UI seperatly.

1. First start up the server. 

        cargo run -- --ip 127.0.0.1 --port 8080

2. Then start the React UI.

        REACT_APP_SERVER_PORT=8080 npm start

### Build

```
npm run build
```