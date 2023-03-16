import React, { useRef } from "react";
import { drawFrontCard } from "./draw/drawCard";

const CardCanvas = props => {
    const canvasRef = useRef(null);

    function draw() {
        const canvas = canvasRef.current;
        if (canvas === null) return;

        const context = canvas.getContext('2d');
        const canvasW = canvas.width;

        drawFrontCard(context, 0, 0, props.value, props.suit, canvasW);
    }

    draw();

    return <canvas ref={canvasRef} className={props.className} width={props.size} height={3 * props.size / 2} />
};

export default CardCanvas;
