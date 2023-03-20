import { roundRect } from "./drawFunctions";
import { newShade } from "./utils";

function drawChip(ctx, x, y, chip_size, color) {
    // Draw chips
    ctx.beginPath();
    ctx.fillStyle = newShade(color, -50);
    ctx.strokeStyle = newShade(color, -100);
    ctx.arc(x, y+0.1*chip_size, chip_size, 0, Math.PI * 2, true);
    ctx.fill();
    ctx.stroke();

    ctx.save();
    ctx.beginPath();
    ctx.fillStyle = color;
    ctx.strokeStyle = newShade(color, -50);
    ctx.arc(x, y, chip_size, 0, Math.PI * 2, true);
    ctx.shadowColor = '#1c1917';
    ctx.shadowBlur = 2;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 2;
    ctx.fill();
    ctx.stroke();
    ctx.restore();

    ctx.beginPath();
    ctx.fillStyle = "white";
    ctx.strokeStyle = newShade(color, -100);
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
    const chipColors = ["#800000", "#800080", "#000000", "#008000", "#FFFF00", "#FFA500", "#FF0000", "#FFFFFF"];

    let remaining = value;
    let totalNumChips = 0;
    for (let i = 0; i < 8; i++) {
        totalNumChips += Math.floor(remaining/chipValues[i]);
        remaining = value % chipValues[i];
    }

    let maxHeight = 3;
    let stacks = Math.floor(totalNumChips / maxHeight);
    stacks = Math.max(stacks, 3);

    remaining = value;
    let y0 = y;
    let stackHeight = maxHeight;
    let stackInd = 0;
    for (let i = 0; i < 8; i++) {
        let chips = Math.floor(remaining/chipValues[i]);
        remaining = value % chipValues[i];

        if (stackHeight <= 0) {
            stackInd = (stackInd + 1) % stacks;
            stackHeight = maxHeight;
            y0 = y;
        }

        for (let j = 0; j < chips; j++) {
            let offsetX = 0;
            let offsetY = 0;
            if (stackInd % 2 === 1) {
                offsetX = 2.4*stackInd*chip_size;
            } else {
                offsetX = 0.4*stackInd*chip_size;
                offsetY = 0.8*stackInd*chip_size;
            }
            
            
            drawChip(ctx, x-w/2 - offsetX, y0 + offsetY, chip_size, chipColors[i]);
            y0 -= 0.5*chip_size;
            stackHeight--;
        }
    }
}