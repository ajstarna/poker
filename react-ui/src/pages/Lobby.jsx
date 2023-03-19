import React from "react";
import { useNavigate } from "react-router-dom";
import MenuButton from "../components/button/MenuButton";
import MenuBody from "../components/layout/MenuBody";

class Lobby extends React.Component {
    constructor(props) {
        super(props);

        this.join = this.join.bind(this);
        this.refresh = this.refresh.bind(this);
    }

    componentDidMount() {
        this.getList();
    }

    join(tableName) {
        const { websocket } = this.props; // websocket instance passed as props to the child component.

        try {
            let data = {
                "msg_type": "join",
                "table_name": tableName,
                "password": ""
            };

            websocket?.send(JSON.stringify(data)); //send data to the server
        } catch (error) {
            console.log(error); // catch error
        }
    }

    refresh(_) {
        this.getList();
    }

    getList() {
        const { websocket } = this.props; // websocket instance passed as props to the child component.

        try {
            let data = {
                "msg_type": "list",
            };

            websocket?.send(JSON.stringify(data)); //send data to the server
        } catch (error) {
            console.log(error); // catch error
        }
    }

    render() {
        console.log(this.props.tables);

        let tableList = [];

        for (const table of Object.values(this.props.tables)) {
            if (table.hasInformation) {
                tableList.push(table);
            }
        }

        return (
            <MenuBody>
                <p className="text-3xl text-gray-200 font-bold mb-5">
                    Lobby
                </p>
                <div className="grid grid-cols-2 gap-4 mt-10">
                    <MenuButton onClick={() => this.props.navigate("/menu")}>Back</MenuButton>
                    <MenuButton onClick={this.refresh} >Refresh List</MenuButton>
                </div>
                <div className="mt-10 shadow-md sm:rounded-lg">
                    {tableList.length > 0 ? (
                        <table className="relative w-full text-sm text-left">
                            <thead className="text-xs uppercase">
                                <tr>
                                    <th className="sticky top-0 px-6 py-3 text-gray-200 bg-gray-800">Table Name</th>
                                    <th className="sticky top-0 px-6 py-3 text-gray-200 bg-gray-800">Blinds</th>
                                    <th className="sticky top-0 px-6 py-3 text-gray-200 bg-gray-800">Buy In</th>
                                    <th className="sticky top-0 px-6 py-3 text-gray-200 bg-gray-800">Players</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y text-stone-200">
                                {
                                    tableList.map((table) => (
                                        <tr onClick={() => this.join(table.tableName)} className="hover:bg-gray-700 active:bg-gray-900">
                                            <td className="px-6 py-4">
                                                <div className="flex flex-row space-x-1">
                                                    {table.tableName}
                                                </div>
                                            </td>
                                            <td className="px-6 py-4">
                                                <div className="flex flex-row space-x-1">
                                                    {table.smallBlind}/{table.bigBlind}
                                                </div>
                                            </td>
                                            <td className="px-6 py-4">
                                                <div className="flex flex-row space-x-1">
                                                    {table.buyIn}
                                                </div>
                                            </td>
                                            <td className="px-6 py-4">
                                                <div className="flex flex-row space-x-1">
                                                    {table.numHumans + table.numBots}/{table.maxPlayers}
                                                </div>
                                            </td>
                                        </tr>
                                    ))
                                }
                            </tbody>
                        </table>

                    ) : (
                        <p className="text-stone-200 p-4 mt-2 w-full bg-gray-700 border-gray-600 border-2 text-center" >
                            There are currenly no public tables listed.
                        </p>
                    )
                    }
                </div>
            </MenuBody>
        );
    }
};

// Wrap and export
/* eslint import/no-anonymous-default-export: [2, {"allowArrowFunction": true}] */
export default (props) => {
    const navigate = useNavigate();

    return <Lobby {...props} navigate={navigate} />;
}

