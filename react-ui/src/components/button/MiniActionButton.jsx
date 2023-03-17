import React from "react";

class MiniActionButton extends React.Component {
    render() {
        return (
            <button {...this.props} className={this.props.className + " text-white bg-stone-700 hover:bg-stone-800 focus:ring-4 focus:outline-none focus:ring-stone-300 font-small rounded-lg text-sm w-auto px-2 py-0.5 text-center dark:bg-stone-600 dark:hover:bg-stone-800 dark:focus:ring-stone-900"}>
                {this.props.children}
            </button>
        );
    }
};

export default MiniActionButton;
