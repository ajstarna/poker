function roundRect(ctx, x, y, w, h, radius)
{
    // because context.roundedRect is not supported on firefox, this is a makeshift version
    var r = x + w;
    var b = y + h;
    ctx.beginPath();
    ctx.moveTo(x+radius, y);
    ctx.lineTo(r-radius, y);
    ctx.quadraticCurveTo(r, y, r, y+radius);
    ctx.lineTo(r, y+h-radius);
    ctx.quadraticCurveTo(r, b, r-radius, b);
    ctx.lineTo(x+radius, b);
    ctx.quadraticCurveTo(x, b, x, b-radius);
    ctx.lineTo(x, y+radius);
    ctx.quadraticCurveTo(x, y, x+radius, y);
}