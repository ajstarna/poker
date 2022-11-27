function roundRect(ctx, x, y, w, h, radius, color)
{
    // because context.roundedRect is not supported on firefox, this is a makeshift version
    var r = x + w;
    var b = y + h;
    ctx.beginPath();
    ctx.fillStyle=color;
    ctx.moveTo(x+radius, y);
    ctx.lineTo(r-radius, y);
    ctx.quadraticCurveTo(r, y, r, y+radius);
    ctx.lineTo(r, y+h-radius);
    ctx.quadraticCurveTo(r, b, r-radius, b);
    ctx.lineTo(x+radius, b);
    ctx.quadraticCurveTo(x, b, x, b-radius);
    ctx.lineTo(x, y+radius);
    ctx.quadraticCurveTo(x, y, x+radius, y);
    ctx.fill();
    ctx.stroke();    
}

function drawBackground(ctx, width, height) {
    // Create gradient
    var x = Math.floor(width/2);
    var y = Math.floor(height/2);
    var r0 = 20;
    var r1 = Math.floor(width/2);

    var grd = ctx.createRadialGradient(x, y, r0, x, y, r1);
    grd.addColorStop(0, "rgb(100, 100, 100)");
    grd.addColorStop(1, "rgb(50, 50, 50)");

    // Fill with gradient
    ctx.fillStyle = grd;
    ctx.fillRect(0, 0, width, height);
}

function drawTable(ctx, width, height) {
    drawBackground(ctx, width, height);

    // Draw table
    // Draw border
    ctx.beginPath();
    ctx.strokeStyle = "rgb(0, 0, 0)";
    var color = "rgb(50, 50, 50)";
    roundRect(ctx, 150, 125, width-300, height-250, 240, color);
    ctx.stroke();
    ctx.fill();
    // Draw cloth
    ctx.beginPath();
    var color2 = "rgb(0, 100, 0)";
    roundRect(ctx, 170, 145, width-340, height-290, 225, color2);
    ctx.stroke();
    ctx.fill();
}
