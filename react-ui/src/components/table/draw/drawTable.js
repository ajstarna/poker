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

    var w = 0.85*size;
    var h = 0.65*size;

    var x0 = cw - w/2;
    var y0 = ch - h/2;

    var x1 = w;
    var y1 = h;

    var borderSize = 0.025 * size;
    var clothSize = 0.04 * size;
    var innerLine = 0.07 * size;

    var r = size / 3 - borderSize;

    // Draw table
    // Draw border
    ctx.save();
    var grdBorder = ctx.createRadialGradient(x0, y0, 20, x0, y0, Math.floor(w/2));
    grdBorder.addColorStop(0, "rgb(100, 100, 100)");
    grdBorder.addColorStop(0.8, "#322F2E");

    ctx.strokeStyle = "rgb(25, 25, 25)";
    ctx.fillStyle = grdBorder;
    roundRect(ctx, x0, y0, x1, y1, r);
    ctx.shadowColor = '#1c1917';
    ctx.shadowBlur = 20;
    ctx.shadowOffsetX = 5;
    ctx.shadowOffsetY = 5;
    ctx.fill();
    ctx.stroke();
    ctx.restore();

    // Draw inner edge
    ctx.save();
    var grdInnerBorder = ctx.createRadialGradient(x0, y0, 20, x0, y0, Math.floor(w/2));
    grdInnerBorder.addColorStop(0, "rgb(50, 50, 50)");
    grdInnerBorder.addColorStop(0.8, "rgb(20, 20, 20)");

    ctx.strokeStyle = "rgb(25, 25, 25)";
    ctx.fillStyle = grdInnerBorder;
    roundRect(ctx, x0 + borderSize, y0 + borderSize, x1 - 2 * borderSize, y1 - 2 * borderSize, r - borderSize);
    ctx.shadowColor = '#1c1917';
    ctx.shadowBlur = 20;
    ctx.shadowOffsetX = 5;
    ctx.shadowOffsetY = 5;
    ctx.fill();
    ctx.stroke();
    ctx.restore();

    // Draw cloth
    ctx.save();
    var grdCloth = ctx.createRadialGradient(cw, 1.1*ch, 20, cw, ch, Math.floor(w/2));
    grdCloth.addColorStop(0, "rgb(0, 150, 0)");
    grdCloth.addColorStop(0.8, "rgb(0, 100, 0)");

    ctx.strokeStyle = "rgb(100, 100, 100)";
    ctx.fillStyle = grdCloth;
    roundRect(ctx, x0 + clothSize, y0 + clothSize, x1 - 2 * clothSize, y1 - 2 * clothSize, r - clothSize);
    ctx.shadowColor = '#1c1917';
    ctx.shadowBlur = 20;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 0;
    ctx.globalCompositeOperation='source-atop';
    ctx.stroke();
    ctx.fill();
    ctx.restore();

    // Draw cloth
    ctx.save();
    ctx.strokeStyle = "rgb(0, 150, 0)";
    roundRect(ctx, x0 + innerLine, y0 + innerLine, x1 - 2 * innerLine, y1 - 2 * innerLine, r - innerLine);
    ctx.stroke();
    ctx.restore();
}