import { roundRect } from "./drawFunctions";

function drawChip(ctx, x, y, chip_size, color) {
    // Draw chips
    ctx.beginPath();
    ctx.fillStyle = color;
    ctx.strokeStyle = "black";
    ctx.arc(x, y+0.1*chip_size, chip_size, 0, Math.PI * 2, true);
    ctx.fill();
    ctx.stroke();

    ctx.beginPath();
    ctx.fillStyle = color;
    ctx.strokeStyle = "black";
    ctx.arc(x, y, chip_size, 0, Math.PI * 2, true);
    ctx.fill();
    ctx.stroke();

    ctx.beginPath();
    ctx.fillStyle = "white";
    ctx.strokeStyle = "black";
    ctx.arc(x, y, 0.6*chip_size, 0, Math.PI * 2, true);
    ctx.fill();
    ctx.stroke();
}

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
    const chipValues = [1000, 500, 100, 25, 20, 10, 5, 1];
    const chipColors = ["maroon", "purple", "black", "green", "yellow", "orange", "red", "white"];

    let remaining = value;
    let y0 = y;
    for (let i = 0; i < 8; i++) {
        let chips = Math.floor(remaining/chipValues[i]);
        remaining = value % chipValues[i];

        for (let j = 0; j < chips; j++) {
            drawChip(ctx, x-w/2, y0, chip_size, chipColors[i]);
            y0 -= 0.4*chip_size;
        }
    }
}