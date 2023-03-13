import React from "react";
import TableCanvas from "../components/table/TableCanvas";
import ActionButton from "../components/button/ActionButton"

class Table extends React.Component {
    constructor(props) {
        super(props);

        this.handleSittingOutChange = this.handleSittingOutChange.bind(this);
    }

    isSittingOut(gameState) {
        if (gameState === null) return true;

        let main_player = gameState.players[gameState.your_index];
        if ("is_sitting_out" in main_player)
            return main_player.is_sitting_out;
        return false;
    }

    handleSittingOutChange(event) {
        const { websocket } = this.props; // websocket instance passed as props to the child component.

        try {
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

            websocket.send(JSON.stringify(data)); //send data to the server
        } catch (error) {
            console.log(error); // catch error
        }
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
                            <input className="leading-tight w-4 h-4" type="checkbox" name="sittingOut"
                                checked={this.isSittingOut(this.props.gameState)}
                                onChange={this.handleSittingOutChange}
                            />
                        </label>
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
                            <input type="range" min="1" max="100" value="1" name="betSizeSlider" className="col-span-2 w-full accent-gray-200 " />
                            <div>
                                <input type="text" name="betSize" className="w-full" />
                            </div>
                        </div>
                        <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-1">
                            <ActionButton>
                                Fold
                            </ActionButton>
                            <ActionButton>
                                Check
                            </ActionButton>
                            <ActionButton>
                                Call
                            </ActionButton>
                            <ActionButton>
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
