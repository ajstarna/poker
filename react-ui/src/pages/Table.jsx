import React, { createRef } from "react";
import { SpeakerWaveIcon, SpeakerXMarkIcon } from "@heroicons/react/24/outline";
import TableCanvas from "../components/table/TableCanvas";
import ActionButton from "../components/button/ActionButton"
import "../components/table/chat.css";
import TextInput from "../components/input/TextInput";
import { handleAdminCommands, ADMIN_PREFIX } from "../utils/admin-actions";
import CardCanvas from "../components/table/CardCanvas";

class Table extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            betSize: 0,
            leaving: false,
            selectedTextWindow: "chat",
            chatMessage: ""
        }

        // Refs
        this.chatEndRef = createRef();

        this.betSlider = createRef();
        this.betSize = createRef();

        // Handlers
        this.handleTextWindowChange = this.handleTextWindowChange.bind(this);

        this.handleMessage = this.handleMessage.bind(this);
        this.handleMessageChange = this.handleMessageChange.bind(this);

        this.handleBetChange = this.handleBetChange.bind(this);
        this.handleSittingOutChange = this.handleSittingOutChange.bind(this);
        this.handleLeave = this.handleLeave.bind(this);

        this.handleFold = this.handleFold.bind(this);
        this.handleCheck = this.handleCheck.bind(this);
        this.handleCall = this.handleCall.bind(this);
        this.handleBet = this.handleBet.bind(this);
    }

    componentDidUpdate(_) {
        this.chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });

        let [min, max] = this.getBetMinMax(this.props.gameState);
        this.betSlider.current.min = min;
        this.betSlider.current.max = max;

        this.betSize.current.min = min;
        this.betSize.current.max = max;

        if (this.state.betSize < min) {
            this.setState({ betSize: min });
        } else if (this.state.betSize > max) {
            this.setState({ betSize: max });
        }
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

    sendToWS(data) {
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

        this.sendToWS(data);
    }

    handleFold(_) {
        let data = {
            "msg_type": "player_action",
            "action": "fold"
        };

        this.sendToWS(data);
    }

    handleCheck(_) {
        let data = {
            "msg_type": "player_action",
            "action": "check"
        };

        this.sendToWS(data);
    }

    handleCall(_) {
        let data = {
            "msg_type": "player_action",
            "action": "call"
        };

        this.sendToWS(data);
    }

    handleBet(_) {
        let data = {
            "msg_type": "player_action",
            "action": "bet",
            "amount": this.state.betSize
        };

        this.sendToWS(data);
    }

    handleBetChange(event) {
        this.setState({ betSize: event.target.value });
    }

    handleLeave(_) {
        this.setState({ leaving: true });
        let data = { "msg_type": "leave" };
        this.sendToWS(data);
    }

    handleTextWindowChange(event) {
        this.setState({ selectedTextWindow: event.target.id });
    }

    handleMessage(_) {
        if (this.state.chatMessage.length > 0) {
            let data = {};
            if (this.state.chatMessage.startsWith(ADMIN_PREFIX)) {
                data = handleAdminCommands(this.state.chatMessage);
            } else {
                data = {
                    "msg_type": "chat",
                    "text": this.state.chatMessage
                };
            }

            console.log(data);

            this.sendToWS(data);

            this.setState({ chatMessage: "" });
        }
    }

    handleMessageChange(event) {
        this.setState({ chatMessage: event.target.value });
    }

    render() {
        let textWindowTab = "inline-block p-4 border-b-2 border-transparent rounded-t-lg hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300 cursor-pointer";
        let textWindowTabActive = "inline-block p-4 text-blue-600 border-b-2 border-blue-600 rounded-t-lg active dark:text-blue-500 dark:border-blue-500 cursor-pointer";

        let cardSize = "24";

        function stringToCards(cardString) {
            var cards = [];
            if (cardString === null) return cards;

            var chars = cardString.split("");

            for (var i = 0; i < chars.length; i += 2) {
                cards.push({
                    value: chars[i],
                    suit: chars[i + 1]
                });
            }

            return (
                <>
                    {
                        cards.map((card) => (
                            <CardCanvas size={cardSize} value={card.value} suit={card.suit} />
                        ))
                    }
                </>
            );
        }

        let playerOrder = [];
        if (this.props.gameState?.players) {
            for (let player of this.props.gameState.players) {
                playerOrder.push({
                    index: player?.index,
                    money: player?.money
                })
            }
        }

        playerOrder.sort((x, y) => y.money - x.money);

        let yourPosition = playerOrder.length;
        for (let i = 0; i < playerOrder.length; i++) {
            if (playerOrder[i].index === this.props.gameState?.your_index) {
                yourPosition = i + 1;
                break;
            }
        }

        let stats = {
            yourPosition: yourPosition,
            numPlayers: playerOrder.length,
            handsPlayed: this.props.gameState?.hands_played
        }

        return (
            <div className="h-screen flex flex-col justify-between">
                <div className="flex-1 flex flex-grow flex-col md:flex-row">
                    <main className="relative flex-1 bg-stone-700">
                        <TableCanvas gameState={this.props.gameState} className="absolute w-full h-full" />
                    </main>

                    <div className="absolute top-0 left-0 p-4">
                        <p className="text-gray-200">
                            Table:  {this.props.gameState && this.props.gameState.name}
                        </p>
                    </div>
                    <div className="absolute bottom-80 left-0 p-4">
                        <label className="block mt-4 mb-2">
                            <span className="text-gray-200 mr-4">
                                Sit Out
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
                                {
                                    this.state.leaving ?
                                        <>
                                            <div role="status" className="flex items-center justify-center">
                                                <span>Leaving...</span>
                                                <svg aria-hidden="true" className="w-4 h-4 ml-4 text-gray-800 animate-spin fill-blue-200" viewBox="0 0 100 101" fill="none" xmlns="http://www.w3.org/2000/svg">
                                                    <path d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z" fill="currentColor" />
                                                    <path d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z" fill="currentFill" />
                                                </svg>
                                            </div>
                                        </>
                                        : "Leave Table"
                                }

                            </ActionButton>
                        </p>
                    </div>

                    <nav className="order-first lg:w-24 xl:w-48 bg-stone-700"></nav>

                    <aside className="lg:w-24 xl:w-48 bg-stone-700"></aside>
                </div>

                <footer className="grid grid-cols-2 h-70 border-t-2 border-stone-600">
                    <div className="bg-stone-700 p-2 flex flex-col">
                        <div className="text-sm font-medium text-center text-gray-500 border-b border-gray-200 dark:text-gray-400 dark:border-gray-700">
                            <ul className="flex flex-wrap -mb-px">
                                <li className="mr-2">
                                    <p id="chat"
                                        onClick={this.handleTextWindowChange}
                                        className={this.state.selectedTextWindow === "chat" ? textWindowTabActive : textWindowTab}>
                                        Chat
                                    </p>
                                </li>
                                <li className="mr-2">
                                    <p id="handHistory"
                                        onClick={this.handleTextWindowChange}
                                        className={this.state.selectedTextWindow === "handHistory" ? textWindowTabActive : textWindowTab}>
                                        Hands
                                    </p>
                                </li>
                                <li className="mr-2">
                                    <p id="stats"
                                        onClick={this.handleTextWindowChange}
                                        className={this.state.selectedTextWindow === "stats" ? textWindowTabActive : textWindowTab}>
                                        Stats
                                    </p>
                                </li>
                            </ul>
                        </div>


                        <div name="chatLog" className="bg-gray-700 text-gray-200 w-full h-40 overflow-scroll scrollbar scrollbar-thumb-gray-100 scrollbar-track-gray-900">
                            {
                                this.state.selectedTextWindow === "chat" &&
                                this.props.chatMessages?.map((message, index) => (
                                    <p key={`chatMessage${index}`} className="text-stone-200 msg">
                                        <strong>{message.user}: </strong>
                                        {message.msg}
                                    </p>
                                ))
                            }
                            {
                                this.state.selectedTextWindow === "handHistory" &&
                                (<>
                                    <div className="shadow-md sm:rounded-lg">
                                        <table className="relative w-full text-sm text-left">
                                            <thead className="text-xs uppercase">
                                                <tr>
                                                    <th className="sticky top-0 px-6 py-3 text-gray-200 bg-gray-800">My Cards</th>
                                                    <th className="sticky top-0 px-6 py-3 text-gray-200 bg-gray-800">Board</th>
                                                    <th className="sticky top-0 px-6 py-3 text-gray-200 bg-gray-800">Winnings</th>
                                                </tr>
                                            </thead>
                                            <tbody className="divide-y">
                                                {
                                                    this.props.handHistory?.map((hand) => (
                                                        <tr>
                                                            <td className="px-6 py-4">
                                                                <div className="flex flex-row space-x-1">
                                                                    {stringToCards(hand.holeCards)}
                                                                </div>
                                                            </td>
                                                            <td className="px-6 py-4">
                                                                <div className="flex flex-row space-x-1">
                                                                    {stringToCards(hand.board)}
                                                                </div>
                                                            </td>
                                                            <td className={`${hand.color} px-6 py-4`}>{hand.winnings}</td>
                                                        </tr>
                                                    ))
                                                }
                                            </tbody>
                                        </table>
                                    </div>
                                </>
                                )
                            }
                            {
                                this.state.selectedTextWindow === "stats" &&
                                (<div className="p-4">
                                    <p>
                                        <strong>Position:</strong> {stats.yourPosition} out of {stats.numPlayers}
                                    </p>
                                    <p className="mt-2">
                                        <strong>Hands Played:</strong> {stats.handsPlayed}
                                    </p>
                                </div>
                                )
                            }
                            <div ref={this.chatEndRef} />
                        </div>
                        <div className="w-full flex flex-row mt-2">
                            < TextInput
                                type="text"
                                name="textMessage"
                                value={this.state.chatMessage}
                                onChange={this.handleMessageChange}
                                onKeyDown={(event) => { if (event.key === 'Enter') this.handleMessage(null); }}
                                className="w-full mr-2 p-2" />
                            <ActionButton className="ml-2" onClick={this.handleMessage}>
                                Send
                            </ActionButton>
                        </div>
                    </div>
                    <div className="bg-stone-700 px-4 md:px-10">
                        <div className="flex flex-col h-full justify-between">
                            <div>
                                <div className="mt-4 grid grid-cols-4 gap-2">
                                    <p className="text-gray-200 font-bold w-full" >Bet:</p>
                                    <input
                                        ref={this.betSlider}
                                        type="range" min="1" max="100"
                                        value={this.state.betSize}
                                        onChange={this.handleBetChange}
                                        name="betSizeSlider"
                                        className="col-span-2 w-full accent-gray-200" />
                                    <input
                                        ref={this.betSize}
                                        type="number" min="0"
                                        value={this.state.betSize}
                                        onChange={this.handleBetChange}
                                        name="betSize"
                                        className="w-full bg-stone-500 text-center text-gray-200 font-bold" />
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
                            <div className="flex flex-row w-full justify-between">
                                <div></div>
                                <ActionButton className="mb-4 x-auto text-gray-200 text-center"
                                    onClick={this.props.soundToggleCallback}>
                                    {this.props.soundEnabled ? <SpeakerXMarkIcon className="w-8 h-8 text-gray-200" /> : <SpeakerWaveIcon className="w-8 h-8 text-gray-200" />}
                                </ActionButton>
                            </div>
                        </div>
                    </div>
                </footer >
            </div >
        );
    }
};

export default Table;
