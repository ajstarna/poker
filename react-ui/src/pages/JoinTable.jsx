import React from "react";
import { useNavigate } from "react-router-dom";
import TextInput from "../components/input/TextInput";
import MenuButton from "../components/button/MenuButton";
import MenuBody from "../components/layout/MenuBody";

class JoinTable extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            tableName: '',
            password: ''
        };

        this.handleChange = this.handleChange.bind(this);
        this.handleSubmit = this.handleSubmit.bind(this);
    }
    handleChange(event) {
        this.setState({
            ...this.state,
            [event.target.name]: event.target.value
        });
    }

    handleSubmit(event) {
        event.preventDefault();

        const { websocket } = this.props; // websocket instance passed as props to the child component.

        try {
            let data = {
                "msg_type": "join",
                "table_name": this.state.tableName,
                "password": this.state.password,
            };

            console.log(data);

            websocket.send(JSON.stringify(data)); //send data to the server
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
                <form onSubmit={this.handleSubmit}>
                    <div className="grid grid-cols-2 gap-4">
                        <div>
                            <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Table Name:</label>
                            <TextInput type="text" name="tableName" value={this.state.tableName} onChange={this.handleChange} placeholder="Table Name" required />
                        </div>
                        <div>
                            <label className="block mt-10 mb-2 text-lg font-medium text-gray-200">Password:</label>
                            <TextInput type="password" name="password" value={this.state.password} onChange={this.handleChange} placeholder="Password" />
                        </div>
                    </div>
                    <MenuButton className="mt-10" type="submit">
                        Join Table
                    </MenuButton>
                </form>
            </MenuBody>
        );
    }
};

// Wrap and export
/* eslint import/no-anonymous-default-export: [2, {"allowArrowFunction": true}] */
export default (props) => {
    const navigate = useNavigate();

    return <JoinTable {...props} navigate={navigate} />;
}

