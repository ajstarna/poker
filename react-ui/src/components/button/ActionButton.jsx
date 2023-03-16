import React from "react";

class ActionButton extends React.Component {
    render() {
        return (
            <button {...this.props} className={this.props.className + " text-white bg-stone-700 hover:bg-stone-800 focus:ring-4 focus:outline-none focus:ring-stone-300 font-medium rounded-lg text-sm w-auto px-5 py-2.5 text-center dark:bg-stone-600 dark:hover:bg-stone-800 dark:focus:ring-stone-900"}>
                {this.props.children}
            </button>
        );
    }
};

export default ActionButton;
