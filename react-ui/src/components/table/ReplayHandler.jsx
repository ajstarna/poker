import React from "react";
import { ChevronLeftIcon, ChevronRightIcon } from "@heroicons/react/24/outline";
import ActionButton from "../button/ActionButton";
import TableCanvas from "./TableCanvas";

class ReplayHandler extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            replayLocation: 0,
        }

        this.decreaseReplayLocation = this.decreaseReplayLocation.bind(this);
        this.incrementReplayLocation = this.incrementReplayLocation.bind(this);
    }

    decreaseReplayLocation(_) {
        let newReplayLocation = this.state.replayLocation - 1;
        if (newReplayLocation < 0) {
            newReplayLocation = 0;
        }

        this.setState({ replayLocation: newReplayLocation });
    }

    incrementReplayLocation(_) {
        let newReplayLocation = this.state.replayLocation + 1;
        if (newReplayLocation >= this.props.replayStateHistory.length) {
            newReplayLocation = this.props.replayStateHistory.length - 1;
        }

        this.setState({ replayLocation: newReplayLocation });
    }

    render() {
        let gameState = null;
        if (this.props.replayStateHistory.length > 0) {
            if (this.state.replayLocation < 0) {
                gameState = this.props.replayStateHistory[0];
            } else if (this.state.replayLocation >= this.props.replayStateHistory.length) {
                let last = this.props.replayStateHistory.length - 1;
                gameState = this.props.replayStateHistory[last];
            } else {
                let index = this.state.replayLocation;
                gameState = this.props.replayStateHistory[index];
            }
        }

        return (
            <div className="flex flex-col">
                <TableCanvas gameState={gameState} className="w-96 h-96" />
                <div className="flex flex-row mt-4">
                    <ActionButton
                        onClick={this.decreaseReplayLocation}
                        value="-1"
                    >
                        <ChevronLeftIcon className="w-4 h-4" />
                    </ActionButton>
                    <div className="grow"></div>
                    <ActionButton
                        onClick={this.incrementReplayLocation}
                        value="1"
                    >
                        <ChevronRightIcon className="w-4 h-4" />
                    </ActionButton>
                </div>
            </div>
        );
    }
};

export default ReplayHandler;
