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
    ctx.strokeStyle = "rgb(0, 0, 0)";
    ctx.fillStyle = "rgb(50, 50, 50)";
    roundRect(ctx, 150, 125, width-300, height-250, 240);
    ctx.stroke();
    ctx.fill();

    // Draw cloth
    ctx.fillStyle = "rgb(0, 100, 0)";
    roundRect(ctx, 170, 145, width-340, height-290, 225);
    ctx.stroke();
    ctx.fill();
}
