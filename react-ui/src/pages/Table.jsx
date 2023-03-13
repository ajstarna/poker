import React, { createRef } from "react";
import TableCanvas from "../components/table/TableCanvas";
import ActionButton from "../components/button/ActionButton"

class Table extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            betSize: 0
        }

        this.betSlider = createRef();
        this.betSize = createRef();

        this.handleBetChange = this.handleBetChange.bind(this);
        this.handleSittingOutChange = this.handleSittingOutChange.bind(this);
        this.handleLeave = this.handleLeave.bind(this);

        this.handleFold = this.handleFold.bind(this);
        this.handleCheck = this.handleCheck.bind(this);
        this.handleCall = this.handleCall.bind(this);
        this.handleBet = this.handleBet.bind(this);
    }

    componentDidUpdate(_) {
        let [min, max] = this.getBetMinMax(this.props.gameState);
        this.betSlider.current.min = min;
        this.betSlider.current.max = max;

        this.betSize.current.min = min;
        this.betSize.current.max = max;
    }

    getBetMinMax(gameState) {
        if (gameState === null) return [0, 0];

        let main_player = gameState.players[gameState.your_index];
        let street_contributions = 0;

        if (gameState.street === "preflop") {
            street_contributions = main_player.preflop_cont;
        } else if (gameState.street === "flop") {
            street_contributions = main_player.flop_cont;
        } else if (gameState.street === "turn") {
            street_contributions = main_player.turn_cont;
        } else if (gameState.street === "river") {
            street_contributions = main_player.river_cont;
        }

        let max = main_player.money + street_contributions
        return [0, max];
    }

    isSittingOut(gameState) {
        if (gameState === null) return true;

        let main_player = gameState.players[gameState.your_index];
        if ("is_sitting_out" in main_player)
            return main_player.is_sitting_out;
        return false;
    }

    sendMessage(data) {
        const { websocket } = this.props; // websocket instance passed as props to the child component.

        try {
            websocket.send(JSON.stringify(data)); //send data to the server
        } catch (error) {
            console.log(error); // catch error
        }
    }

    handleSittingOutChange(event) {
        let data = {};
        if (event.target.checked) {
            data = {
                "msg_type": "sitout",
            };
        } else {
            data = {
                "msg_type": "imback",
            };
        }

        this.sendMessage(data);
    }

    handleFold(_) {
        let data = {
            "msg_type": "player_action",
            "action": "fold"
        };

        this.sendMessage(data);
    }

    handleCheck(_) {
        let data = {
            "msg_type": "player_action",
            "action": "check"
        };

        this.sendMessage(data);
    }

    handleCall(_) {
        let data = {
            "msg_type": "player_action",
            "action": "call"
        };

        this.sendMessage(data);
    }

    handleBet(_) {
        let data = {
            "msg_type": "player_action",
            "action": "bet",
            "amount": this.state.betSize
        };

        this.sendMessage(data);
    }

    handleBetChange(event) {
        this.setState({ betSize: event.target.value });
    }

    handleLeave(_) {
        let data = { "msg_type": "leave" };
        this.sendMessage(data);
    }

    render() {
        return (
            <div className="h-screen flex flex-col justify-between">
                <div className="flex-1 flex flex-grow flex-col md:flex-row">
                    <main className="relative flex-1 bg-stone-700">
                        <TableCanvas gameState={this.props.gameState} className="absolute w-full h-full" />
                    </main>

                    <div className="absolute p-4">
                        <p className="text-gray-200">
                            Table:  {this.props.gameState && this.props.gameState.name}
                        </p>
                        <label className="block mt-4 mb-2">
                            <span className="text-gray-200 mr-4">
                                Sitting Out
                            </span>
                            <input className="leading-tight w-4 h-4 accent-gray-200" type="checkbox" name="sittingOut"
                                checked={this.isSittingOut(this.props.gameState)}
                                onChange={this.handleSittingOutChange}
                            />
                        </label>
                    </div>

                    <div className="absolute p-4 top-0 right-0">
                        <p className="text-gray-200">
                            <ActionButton onClick={this.handleLeave}>
                                Leave Table
                            </ActionButton>
                        </p>
                    </div>

                    <nav className="order-first lg:w-24 xl:w-48 bg-stone-700"></nav>

                    <aside className="lg:w-24 xl:w-48 bg-stone-700"></aside>
                </div>

                <footer className="grid grid-cols-2 h-40 border-t-2 border-stone-600">
                    <div className="bg-stone-700">
                        Chat
                    </div>
                    <div className="bg-stone-700 px-4 md:px-10">
                        <div className="mt-4 grid grid-cols-4 gap-2">
                            <div className="text-gray-200 w-full" >Bet:</div>
                            <input
                                ref={this.betSlider}
                                type="range" min="1" max="100"
                                value={this.state.betSize}
                                onChange={this.handleBetChange}
                                name="betSizeSlider"
                                className="col-span-2 w-full accent-gray-200 " />
                            <div>
                                <input
                                    ref={this.betSize}
                                    type="number" min="0"
                                    value={this.state.betSize}
                                    onChange={this.handleBetChange}
                                    name="betSize"
                                    className="w-full" />
                            </div>
                        </div>
                        <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-1">
                            <ActionButton onClick={this.handleFold}>
                                Fold
                            </ActionButton>
                            <ActionButton onClick={this.handleCheck}>
                                Check
                            </ActionButton>
                            <ActionButton onClick={this.handleCall}>
                                Call
                            </ActionButton>
                            <ActionButton onClick={this.handleBet}>
                                Bet
                            </ActionButton>
                        </div>
                    </div>
                </footer>
            </div>
        );
    }
};

export default Table;
