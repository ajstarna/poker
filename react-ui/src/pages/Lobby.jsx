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
        return (
            <MenuBody>
                <p className="text-3xl text-gray-200 font-bold mb-5">
                    Lobby
                </p>
                <MenuButton className="mt-10" onClick={this.refresh} >
                    Refresh List
                </MenuButton>
                <div className="mt-10">
                    {this.props.tables?.map((table) => (
                        <p key={table} onClick={() => this.join(table)} className="text-stone-200 p-4 mt-2 w-full bg-gray-700 hover:bg-gray-800 active:bg-gray-900 border-gray-600 border-2">
                            {table}
                        </p>
                    ))}
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

