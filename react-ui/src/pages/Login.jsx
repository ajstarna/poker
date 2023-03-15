import React from "react";
import { useNavigate } from "react-router-dom";
import MenuButton from "../components/button/MenuButton";
import MenuBody from "../components/layout/MenuBody";
import TextInput from "../components/input/TextInput";

class Login extends React.Component {
    constructor(props) {
        super(props);
        this.state = { value: '' };

        this.handleChange = this.handleChange.bind(this);
        this.handleSubmit = this.handleSubmit.bind(this);
    }

    componentDidMount() {
        const { player_name } = this.props // name passed as props to the child component.

        console.log(player_name);

        if (player_name) {
            this.setState({ value: player_name });
        }
    }

    handleChange(event) {
        this.setState({ value: event.target.value });
    }

    handleSubmit(event) {
        console.log('A name was submitted: ' + this.state.value);
        event.preventDefault();

        const { websocket } = this.props // websocket instance passed as props to the child component.

        try {
            let data = {
                "msg_type": "name",
                "player_name": this.state.value
            };

            websocket.send(JSON.stringify(data)); //send data to the server
            this.props.navigate("/menu");
        } catch (error) {
            console.log(error) // catch error
        }
    }

    render() {
        return (
            <MenuBody>
                <p className="text-3xl text-gray-200 font-bold mb-5">
                    Good Friends Poker
                </p>
                <p className="text-gray-200 text-lg">
                    Please enter your name.
                </p>
                <form onSubmit={this.handleSubmit}>
                    <label htmlFor="username" className="block mt-10 mb-2 text-lg font-medium text-gray-200">Username:</label>
                    <TextInput type="text" id="username" value={this.state.value} onChange={this.handleChange} placeholder="Username" required />
                    <MenuButton className="mt-10" type="submit">Next</MenuButton>
                </form>
            </MenuBody>
        );
    }
};

// Wrap and export
/* eslint import/no-anonymous-default-export: [2, {"allowArrowFunction": true}] */
export default (props) => {
    const navigate = useNavigate();

    return <Login {...props} navigate={navigate} />;
}

