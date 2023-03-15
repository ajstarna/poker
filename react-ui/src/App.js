import React, { createRef }  from 'react';
import { Routes, Route, useNavigate } from "react-router-dom";

import MenuBody from './components/layout/MenuBody';
import ErrorModal from './components/modal/ErrorModal';
import Spinner from './components/spinnner/Spinner';
import Create from './pages/Create';
import JoinTable from './pages/JoinTable';
import Lobby from './pages/Lobby';
import Login from "./pages/Login";
import Menu from './pages/Menu';
import Table from "./pages/Table";

class App extends React.Component {
  constructor(props) {
    super(props);

    const playerName = localStorage.getItem('poker-player-name');

    this.state = {
        ws: null,
        playerName: playerName,
        creatingTable: false,
        gameState: null,
        soundEnabled: false,
        chatMessages: [],
        handHistory: [],
        logMessages: [],
        tables: [],
        showErrorModal: false,
        errorMessage: ''
    };

    this.deckSuffleSound = createRef();
    this.notificationActionSound = createRef();

    this.soundToggleCallback = this.soundToggleCallback.bind(this);
  }

  // single websocket instance for the own application and constantly trying to reconnect.

  componentDidMount() {
    this.connect();
  }

  timeout = 250; // Initial timeout duration as a class variable

  /**
   * @function connect
   * This function establishes the connect with the websocket and also ensures constant reconnection if connection closes
   */
  connect = () => {
    const proto = window.location.protocol.startsWith("https") ? "wss" : "ws";
    const hostname = window.location.hostname;
    const uuid = localStorage.getItem('poker-uuid');

    let port = "";
    if (process.env.REACT_APP_SERVER_PORT) {
      port = `:${process.env.REACT_APP_SERVER_PORT}`;
      console.log(`REACT_APP_SERVER_PORT=${process.env.REACT_APP_SERVER_PORT}`);
    }

    let wsUri = `${proto}://${hostname}${port}/join`;
    if (uuid) {
      wsUri = `${proto}://${hostname}${port}/rejoin/${uuid}`;
    }

    const ws = new WebSocket(wsUri);
    let that = this; // cache the this
    var connectInterval;

    // websocket onopen event listener
    ws.onopen = () => {
      console.log("connected websocket App component");

      this.setState({ ws: ws });

      that.timeout = 250; // reset timer to 250 on open of websocket connection 
      clearTimeout(connectInterval); // clear Interval on on open of websocket connection
    };

     // websocket onclose event listener
     ws.onclose = e => {
      console.log(
          `Socket is closed. Reconnect will be attempted in ${Math.min(
              10000 / 1000,
              (that.timeout + that.timeout) / 1000
          )} second.`,
          e.reason
      );

      that.timeout = that.timeout + that.timeout; //increment retry interval
      connectInterval = setTimeout(this.check, Math.min(10000, that.timeout)); //call check function after timeout
    };

    // websocket onerror event listener
    ws.onerror = err => {
        console.error(
            "Socket encountered error: ",
            err.message,
            "Closing socket"
        );

        ws.close();
    };

    ws.onmessage = function (event) {
      const json = JSON.parse(event.data);
      try {
        console.log(json);
  
        if (json.msg_type === "connected") {
          console.log(`Connected with UUID: ${json.uuid}`);
          localStorage.setItem('poker-uuid', json.uuid);
        } else if (json.msg_type === "player_name") {
          console.log(`New player name: ${json.player_name}`);
          if (json.player_name) {
            that.setState({ playerName: json.player_name });
            localStorage.setItem('poker-player-name', json.player_name);
          } else if (that.state.playerName) {
            let data = {
              "msg_type": "name",
              "player_name": that.state.playerName
            };

            ws.send(JSON.stringify(data)); //send data to the server
          }
        } else if (json.msg_type === "tables_list") {
          that.setState({ tables: json.tables });
        } else if (json.msg_type === "created_game") {
          let output = "You created a game. Type '/help' for a list of available admin commands. (Private games only)";
          that.log(output);	
          that.setState({creatingTable: false});
          that.props.navigate("/table");
        } else if (json.msg_type === "game_state") {
          that.setState({
            creatingTable: false,
            gameState: json
          });
          that.props.navigate("/table");
        } else if (json.msg_type === "chat") {
          let output = json.player_name + ": " + json.text
          that.chat(output);
        } else if (json.msg_type === "new_hand") {
          if (that.state.soundEnabled) {
            that.deckSuffleSound.current?.play();
          }
          that.log("Playing hand " + json.hand_num);
        } else if (json.msg_type === "prompt") {
          if (that.state.soundEnabled) {
            that.notificationActionSound.current?.play();
          }
          that.log("Your turn to act ...");
        } else if (json.msg_type === "finish_hand") {
          that.saveHandHistory(json.pay_outs);
        } else if (json.msg_type === "left_game") {
          that.props.navigate("/menu");
        } else if (json.msg_type === "error") {
          if (json.error === "unable_to_create") {
            that.setState({creatingTable: false});
          }

          that.setState({
            showErrorModal: true,
            errorMessage: json.error + ": " + json.reason
          });
        }
      } catch (err) {
        console.log(err);
      }
    };
  };

  /**
   * utilited by the @function connect to check if the connection is close, if so attempts to reconnect
   */
  check = () => {
      const { ws } = this.state;
      if (!ws || ws.readyState === WebSocket.CLOSED) this.connect(); //check if websocket instance is closed, if so call `connect` function.
  };

  chat(msg) {
    this.setState({ 
      chatMessages: [...this.state.chatMessages, msg]
    });
  }

  saveHandHistory(payOuts) {
    let { gameState } = this.state;

    let playerIndex = gameState.your_index;
    let holeCards = gameState.hole_cards;
    let board = "";

    if ("flop" in gameState) {
      board += gameState.flop;
    }

    if ("turn" in gameState) {
      board += gameState.turn;
    }

    if ("river" in gameState) {
      board += gameState.river;
    }

    let winnings = 0;
    let player = gameState.players[playerIndex];
    
    for (let payOut of payOuts) {
      if (payOut.index === playerIndex) {
        winnings = payOut.payout;
        break;
      }
    }

    if ("preflop_cont" in player) {
      winnings -= player.preflop_cont;
    }

    if ("flop_cont" in player) {
      winnings -= player.flop_cont;
    }

    if ("turn_cont" in player) {
      winnings -= player.turn_cont;
    }

    if ("river_cont" in player) {
      winnings -= player.river_cont;
    }

    let color = "text-gray-200";

    if (winnings > 0) {
      color = "text-green-500";
    } else if (winnings < 0) {
      color = "text-red-500";
    }

    let history = {
      holeCards: holeCards,
      board: board,
      winnings: Math.abs(winnings),
      loss: winnings < 0,
      color: color
    }

    this.setState({ 
      handHistory: [...this.state.handHistory, history]
    });
  }

  log(msg) {
    console.log(msg);
    this.setState({ 
      logMessages: [...this.state.logMessages, msg]
    });
  }

  soundToggleCallback(event) {
    this.setState({ soundEnabled: !this.state.soundEnabled });
  }

  render() {
    return (
      <>
      {this.state.showErrorModal ? (
        <ErrorModal onClick={() => this.setState({ showErrorModal: false })}>
          {this.state.errorMessage}
        </ErrorModal>
      ) : null}
        <Routes>
          <Route path="/" element={<Login websocket={this.state.ws} player_name={this.state.playerName} />} />
          <Route path="/menu" element={<Menu />} />
          <Route path="/join" element={<JoinTable websocket={this.state.ws} />} />
          <Route path="/lobby" element={<Lobby websocket={this.state.ws} tables={this.state.tables} />} />
          <Route path="/create" element={
            this.state.creatingTable ? (
              <MenuBody>
                <Spinner>
                  Creating Table...
                </Spinner>
              </MenuBody>
            ) :
            (
              <Create websocket={this.state.ws} onCreate={() => this.setState({creatingTable: true})}/>
            )
          } />
          <Route path="/table" element={
            <Table 
              websocket={this.state.ws} 
              gameState={this.state.gameState} 
              soundEnabled={this.state.soundEnabled} 
              soundToggleCallback={this.soundToggleCallback} 
              chatMessages={this.state.chatMessages}
              handHistory={this.state.handHistory}
              logMessages={this.state.logMessages}
              />} />
        </Routes>
        <audio ref={this.deckSuffleSound} src={process.env.PUBLIC_URL + '/assets/sounds/cards-shuffling.mp3'} preload="auto" controls="none" className="hidden" />
        <audio ref={this.notificationActionSound} src={process.env.PUBLIC_URL + '/assets/sounds/notification-action.mp3'} preload="auto" controls="none" className="hidden" />
        </>
    );
  };
}

// Wrap and export
/* eslint import/no-anonymous-default-export: [2, {"allowArrowFunction": true}] */
export default (props) => {
  const navigate = useNavigate();
  
  return <App {...props} navigate={navigate} />;
}

