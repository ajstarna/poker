import React from "react";

const clamp = (num, min, max) => Math.min(Math.max(num, min), max);

class Counter extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            value: parseInt(this.props.value)
        };

        this.handleDecrement = this.handleDecrement.bind(this);
        this.handleIncrement = this.handleIncrement.bind(this);
        this.handleChange = this.handleChange.bind(this);
        this.getValue = this.getValue.bind(this);
    }

    getValue() {
        return parseInt(this.state.value);
    }

    handleDecrement(_) {
        let newValue = this.state.value - parseInt(this.props.step);
        newValue = clamp(newValue, this.props.min, this.props.max);
        this.setState({
            value: newValue
        });
    }

    handleIncrement(_) {
        let newValue = this.state.value + parseInt(this.props.step);
        newValue = clamp(newValue, this.props.min, this.props.max);
        this.setState({
            value: newValue
        });
    }

    handleChange(event) {
        let newValue = event.target.value;
        if (newValue.length > 0) {
            newValue = parseInt(newValue) || 0;

            this.setState({
                value: clamp(newValue, this.props.min, this.props.max)
            });
        } else {
            this.setState({ value: "" });
        }
    }

    render() {
        return (
            <div className="flex flex-row h-10 w-full rounded-lg relative bg-transparent mt-1">
                <button
                    type="button"
                    data-action="decrement"
                    className=" bg-gray-700 text-white hover:text-gray-400 hover:bg-gray-600 h-full w-20 rounded-l cursor-pointer outline-none"
                    onClick={this.handleDecrement}
                >
                    <span className="m-auto text-2xl font-thin">−</span>
                </button>
                <input
                    type="number"
                    className="outline-none focus:outline-none text-center w-full bg-gray-700 font-semibold text-md hover:text-gray-400 focus:text-gray-400  md:text-basecursor-default flex items-center text-white outline-none"
                    name="custom-input-number"
                    min={this.props.min}
                    max={this.props.max}
                    value={this.state.value}
                    onChange={this.handleChange}
                ></input>
                <button
                    type="button"
                    data-action="increment"
                    className="bg-gray-700 text-white hover:text-gray-400 hover:bg-gray-600 h-full w-20 rounded-r cursor-pointer"
                    onClick={this.handleIncrement}
                >
                    <span className="m-auto text-2xl font-thin">+</span>
                </button>
            </div>
        );
    }
};

export default Counter;
