import React from "react";
import { useNavigate } from "react-router-dom";
import MenuButton from "../components/button/MenuButton";
import TextInput from "../components/input/TextInput";
import MenuBody from "../components/layout/MenuBody";

class Create extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            maxPlayers: 2,
            numBots: 0,
            smallBlind: 1,
            bigBlind: 2,
            startingStack: 1000,
            private: false,
            password: ""
        };

        this.handleChange = this.handleChange.bind(this);
        this.handleSubmit = this.handleSubmit.bind(this);
    }

    componentDidMount() {
    }

    handleChange(event) {
        let value = event.target.value;
        const type = event.target.type;

        if (type === "number") {
            value = parseInt(value);
        }

        if (type === "checkbox") {
            value = event.target.checked;
        }

        this.setState({
            ...this.state,
            [event.target.name]: value
        });
    }

    handleSubmit(event) {
        event.preventDefault();

        const { websocket } = this.props; // websocket instance passed as props to the child component.

        try {
            let data = {
                "msg_type": "create",
                "max_players": this.state.maxPlayers,
                "num_bots": this.state.numBots,
                "small_blind": this.state.smallBlind,
                "big_blind": this.state.bigBlind,
                "buy_in": this.state.startingStack,
            };

            if (this.state.private) {
                data["password"] = this.state.password;
            }

            websocket.send(JSON.stringify(data)); //send data to the server
            this.props.onCreate();
        } catch (error) {
            console.log(error); // catch error
        }
    }

    render() {
        return (
            <MenuBody>
                <p className="text-3xl text-gray-200 font-bold mb-5">
                    Create Table
                </p>
                <p className="text-gray-200 text-lg">
                    Please enter the following information.
                </p>
                <form onSubmit={this.handleSubmit}>
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Max Players:</label>
                    <TextInput type="number" min="2" max="9" step="1" name="maxPlayers" value={this.state.maxPlayers} onChange={this.handleChange} required />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Number of Bots:</label>
                    <TextInput type="number" min="0" max="8" step="1" name="numBots" value={this.state.numBots} onChange={this.handleChange} required />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Small Blind:</label>
                    <TextInput type="number" min="1" step="1" name="smallBlind" value={this.state.smallBlind} onChange={this.handleChange} required />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Big Blind:</label>
                    <TextInput type="number" min="1" step="1" name="bigBlind" value={this.state.bigBlind} onChange={this.handleChange} required />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Starting Stack:</label>
                    <TextInput type="number" min="0" step="100" name="startingStack" value={this.state.startingStack} onChange={this.handleChange} required />
                    <label className="block mt-10 mb-2">
                        <input className="mr-4 leading-tight w-4 h-4" type="checkbox" name="private" value={this.state.private} onChange={this.handleChange} />
                        <span className="text-lg font-medium text-gray-200">
                            Private
                        </span>
                    </label>
                    {this.state.private &&
                        <>
                            <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Password:</label>
                            <TextInput type="password" name="password" value={this.state.password} onChange={this.handleChange} required />
                        </>
                    }
                    <MenuButton type="submit" className="mt-10">Create</MenuButton>
                </form>
            </MenuBody>
        );
    }
};

// Wrap and export
/* eslint import/no-anonymous-default-export: [2, {"allowArrowFunction": true}] */
export default (props) => {
    const navigate = useNavigate();

    return <Create {...props} navigate={navigate} />;
}

