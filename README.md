# poker

A poker implementation written in Rust.

The IO is implemented as a websocket server using the Actix framework.

You can run it inside a docker container via:

```
docker build --tag poker --file Dockerfile .
```

and then

```
docker run -p 8080:8080 poker
```

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

        cd react-ui
        REACT_APP_SERVER_PORT=8080 npm start

### Build UI

You can simply call the following.

```
cd react-ui
npm run build
```

If you need to specify the port of your server then you can run the following build command.

```
cd react-ui
REACT_APP_SERVER_PORT=8080 npm run build
```
