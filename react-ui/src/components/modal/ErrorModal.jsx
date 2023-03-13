import React from "react";

class ErrorModal extends React.Component {
    render() {
        return (
            <>
                <div
                    className="justify-center items-center flex overflow-x-hidden overflow-y-auto fixed inset-0 z-50 outline-none focus:outline-none"
                >
                    <div className="relative w-auto my-6 mx-auto max-w-3xl">
                        {/*content*/}
                        <div className="border-0 rounded-lg shadow-lg relative flex flex-col w-full bg-gray-900 outline-none focus:outline-none">
                            {/*header*/}
                            <div className="flex items-start justify-between p-5 border-b border-solid border-gray-600 rounded-t">
                                <h3 className="text-3xl font-semibold text-gray-200">
                                    Error
                                </h3>
                                <button
                                    className="p-1 ml-auto bg-transparent border-0 text-gray-200 opacity-5 float-right text-3xl leading-none font-semibold outline-none focus:outline-none"
                                    onClick={this.props.onClick}
                                >
                                    <span className="bg-transparent text-gray-200 opacity-5 h-6 w-6 text-2xl block outline-none focus:outline-none">
                                        ×
                                    </span>
                                </button>
                            </div>
                            {/*body*/}
                            <div className="relative p-6 flex-auto">
                                <p className="my-4 text-gray-200 text-lg leading-relaxed">
                                    {this.props.children}
                                </p>
                            </div>
                            {/*footer*/}
                            <div className="flex items-center justify-end p-6 border-t border-solid border-gray-600 rounded-b">
                                <button
                                    className="text-red-800 background-transparent font-bold uppercase px-6 py-2 text-sm outline-none focus:outline-none mr-1 mb-1 ease-linear transition-all duration-150"
                                    type="button"
                                    onClick={this.props.onClick}
                                >
                                    Close
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
                <div className="opacity-25 fixed inset-0 z-40 bg-black"></div>
            </>
        );
    };
}

export default ErrorModal;