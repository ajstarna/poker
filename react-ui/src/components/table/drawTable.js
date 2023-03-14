import { roundRect } from "./drawFunctions";

export function drawBackground(ctx, width, height) {
    // Create gradient
    var x = Math.floor(width / 2);
    var y = Math.floor(height / 2);
    var r0 = 20;
    var r1 = Math.floor(width / 2);

    var grd = ctx.createRadialGradient(x, y, r0, x, y, r1);
    grd.addColorStop(0, "rgb(100, 100, 100)");
    grd.addColorStop(0.8, "#44403c");

    // Fill with gradient
    ctx.fillStyle = grd;
    ctx.fillRect(0, 0, width, height);
}

export function drawTable(ctx, width, height) {
    var size = Math.min(width, height);

    var cw = width/2;
    var ch = height/2;

    var w = 0.8*size;
    var h = 0.6*size;

    var x0 = cw - w/2;
    var y0 = ch - h/2;

    var x1 = w;
    var y1 = h;

    var borderSize = 0.025 * size;

    var r = size / 3 - borderSize;

    // Draw table
    // Draw border
    ctx.strokeStyle = "rgb(0, 0, 0)";
    ctx.fillStyle = "#1c1917";
    roundRect(ctx, x0, y0, x1, y1, r);
    ctx.stroke();
    ctx.fill();

    // Draw cloth
    ctx.fillStyle = "rgb(0, 100, 0)";
    roundRect(ctx, x0 + borderSize, y0 + borderSize, x1 - 2 * borderSize, y1 - 2 * borderSize, r - borderSize);
    ctx.stroke();
    ctx.fill();
}