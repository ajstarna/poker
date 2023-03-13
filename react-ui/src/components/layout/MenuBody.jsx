import React from "react";

class MenuBody extends React.Component {
    render() {
        return (
            <div className="container max-w-xl mx-auto bg-gray-800 rounded-xl shadow border p-8 m-10">
                {this.props.children}
            </div>
        );
    }
};

export default MenuBody;
