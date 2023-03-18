import { roundRect } from "./drawFunctions";

export function drawChips(ctx, value, x, y, w, h, size) {
    let chip_size = 0.15*size;

    // Draw boarder for text
    ctx.fillStyle = "#00000066";
    ctx.strokeStyle = "black";
    roundRect(ctx, x-w/2, y-h/2, w, h, 5);
    ctx.stroke();
    ctx.fill();

    // Draw text (street contributions)
    ctx.font = `${0.2*size}px arial`;
    ctx.textAlign = "start";
    ctx.fillStyle = "white";
    ctx.fillText(value, x-w/2 + 0.3*size, y-h/2 + 0.2*size);

    // Draw chips
    ctx.beginPath();
    ctx.fillStyle = "yellow";
    ctx.strokeStyle = "black";
    ctx.arc(x-w/2, y+0.1*chip_size, chip_size, 0, Math.PI * 2, true);
    ctx.fill();
    ctx.stroke();

    ctx.beginPath();
    ctx.fillStyle = "yellow";
    ctx.strokeStyle = "black";
    ctx.arc(x-w/2, y, chip_size, 0, Math.PI * 2, true);
    ctx.fill();
    ctx.stroke();

    ctx.beginPath();
    ctx.fillStyle = "white";
    ctx.strokeStyle = "black";
    ctx.arc(x-w/2, y, 0.6*chip_size, 0, Math.PI * 2, true);
    ctx.fill();
    ctx.stroke();
}