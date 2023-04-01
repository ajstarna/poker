import React, { createRef } from "react";
import { useNavigate } from "react-router-dom";
import MenuButton from "../components/button/MenuButton";
import TextInput from "../components/input/TextInput";
import Counter from "../components/input/Counter";
import MenuBody from "../components/layout/MenuBody";

class Create extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            private: false,
            password: ""
        };

        this.maxPlayersRef = createRef();
        this.numBotsRef = createRef();
        this.smallBlindRef = createRef();
        this.bigBlindRef = createRef();
        this.buyInRef = createRef();

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
                "max_players": this.maxPlayersRef.current.getValue(),
                "num_bots": this.numBotsRef.current.getValue(),
                "small_blind": this.smallBlindRef.current.getValue(),
                "big_blind": this.bigBlindRef.current.getValue(),
                "buy_in": this.buyInRef.current.getValue(),
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
                    <Counter ref={this.maxPlayersRef} min="2" max="9" step="1" name="maxPlayers" value="9" />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Number of Bots:</label>
                    <Counter ref={this.numBotsRef} min="0" max="8" step="1" name="numBots" value="0" />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Small Blind:</label>
                    <Counter ref={this.smallBlindRef} min="0" max="10000" step="1" name="smallBlind" value="1" />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Big Blind:</label>
                    <Counter ref={this.bigBlindRef} min="0" max="10000" step="1" name="bigBlind" value="2" />
                    <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Buy In:</label>
                    <Counter ref={this.buyInRef} min="0" max="10000" step="100" name="buyIn" value="200" />
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
                    <div className="grid grid-cols-2 gap-4 mt-10">
                        <MenuButton type="button" onClick={() => this.props.navigate("/menu")}>Back</MenuButton>
                        <MenuButton type="submit">Create</MenuButton>
                    </div>

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

