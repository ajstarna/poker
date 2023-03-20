import { roundRect } from "./drawFunctions";
import { newShade } from "./utils";

function drawCardBase(ctx, x, y, width, height, color) {
    ctx.save();
    var grd = ctx.createRadialGradient(x + width/2, y + height/2, 5, x + width, y + height, 2*width);
    grd.addColorStop(0, color);
    grd.addColorStop(0.8, newShade(color, -100));

    // Draw using 5px for border radius on all sides
    // stroke it but no fill
    ctx.fillStyle = grd;
    ctx.strokeStyle = newShade(color, 100);
    roundRect(ctx, x, y, width, height, 5);
    ctx.shadowColor = '#1c1917';
    ctx.shadowBlur = 5;
    ctx.shadowOffsetX = 2;
    ctx.shadowOffsetY = 2;
    ctx.fill();
    ctx.stroke();
    ctx.restore();
}

function drawClub(ctx, x, y, width, height, color) {
    let circleRadius = width * 0.3;
    let bottomWidth = width * 0.5;
    ctx.fillStyle = color;

    // top circle
    ctx.beginPath();
    ctx.arc(x, y + circleRadius + (height * 0.05), 
                circleRadius, 0, 2 * Math.PI, false
    );
    ctx.fill();
    
    // bottom right circle
    ctx.beginPath();
    ctx.arc(x + circleRadius, y + (height * 0.6), 
                circleRadius, 0, 2 * Math.PI, false
    );
    ctx.fill();
    
    // bottom left circle
    ctx.beginPath();
    ctx.arc(x - circleRadius, y + (height * 0.6), 
                circleRadius, 0, 2 * Math.PI, false
    );
    ctx.fill();
    
    // center filler circle
    ctx.beginPath();
    ctx.arc(x, y + (height * 0.5), 
                circleRadius / 2, 0, 2 * Math.PI, false
    );
    ctx.fill();
    
    // bottom of club
    ctx.moveTo(x, y + (height * 0.6));
    ctx.quadraticCurveTo(x, y + height, 
                             x - bottomWidth / 2, y + height
    );
    ctx.lineTo(x + bottomWidth / 2, y + height);
    ctx.quadraticCurveTo(x, y + height, 
                             x, y + (height * 0.6)
    );
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
}

function drawDiamond(ctx, x, y, width, height, color){
    ctx.beginPath();
    ctx.moveTo(x, y);

    // top left edge
    ctx.lineTo(x - width / 2, y + height / 2);

    // bottom left edge
    ctx.lineTo(x, y + height);

    // bottom right edge
    ctx.lineTo(x + width / 2, y + height / 2);

    // closing the path automatically creates
    // the top right edge
    ctx.closePath();

    ctx.fillStyle = color;
    ctx.fill();
}

function drawHeart(ctx, x, y, width, height, color) {
    ctx.beginPath();
    let topCurveHeight = height * 0.3;
    ctx.moveTo(x, y + topCurveHeight);
    // top left curve
    ctx.bezierCurveTo(x, y, 
        x - width / 2, y, 
        x - width / 2, y + topCurveHeight
    );

    // bottom left curve
    ctx.bezierCurveTo(x - width / 2, y + (height + topCurveHeight) / 2, 
        x, y + 1.25 * (height + topCurveHeight) / 2, 
        x, y + height
    );

    // bottom right curve
    ctx.bezierCurveTo(x, y + 1.25 * (height + topCurveHeight) / 2, 
        x + width / 2, y + (height + topCurveHeight) / 2, 
        x + width / 2, y + topCurveHeight
    );

    // top right curve
    ctx.bezierCurveTo(x + width / 2, y, 
        x, y, 
        x, y + topCurveHeight
    );

    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
}

function drawSpade(ctx, x, y, width, height, color){
    var bottomWidth = width * 0.7;
    var topHeight = height * 0.7;
    var bottomHeight = height * 0.3;

    ctx.beginPath();
    ctx.moveTo(x, y);

    // top left of spade          
    ctx.bezierCurveTo(
        x, y + topHeight / 2, // control point 1
        x - width / 2, y + topHeight / 2, // control point 2
        x - width / 2, y + topHeight // end point
    );

    // bottom left of spade
    ctx.bezierCurveTo(
        x - width / 2, y + topHeight * 1.3, // control point 1
        x, y + topHeight * 1.3, // control point 2
        x, y + topHeight // end point
    );

    // bottom right of spade
    ctx.bezierCurveTo(
        x, y + topHeight * 1.3, // control point 1
        x + width / 2, y + topHeight * 1.3, // control point 2
        x + width / 2, y + topHeight // end point
    );

    // top right of spade
    ctx.bezierCurveTo(
        x + width / 2, y + topHeight / 2, // control point 1
        x, y + topHeight / 2, // control point 2
        x, y // end point
    );

    ctx.closePath();
    ctx.fill();

    // bottom of spade
    ctx.beginPath();
    ctx.moveTo(x, y + topHeight);
    ctx.quadraticCurveTo(
        x, y + topHeight + bottomHeight, // control point
        x - bottomWidth / 2, y + topHeight + bottomHeight // end point
    );
    ctx.lineTo(x + bottomWidth / 2, y + topHeight + bottomHeight);
    ctx.quadraticCurveTo(
        x, y + topHeight + bottomHeight, // control point
        x, y + topHeight // end point
    );
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
}

function getSuitColor(suit) {
    if (suit === 'c')
        return "rgb(0, 150, 0)";
    if (suit === 's')
        return "rgb(30, 30, 30)";
    if (suit === 'h')
        return "rgb(200, 0, 0)";
    if (suit === 'd')
        return "rgb(0, 0, 255)";

    throw new Error('Suit must be one of the following [c, s, h, d]');
}

function drawSuit(ctx, x, y, size, suit, color) {
    if (suit === 'c') {
        drawClub(ctx, x, y, size, 3*size/2, color);
        return;
    }

    if (suit === 's') {
        drawSpade(ctx, x, y, size, 3*size/2, color);
        return;
    }

    if (suit === 'h') {
        drawHeart(ctx, x, y, size, 3*size/2, color);
        return;
    }

    if (suit === 'd') {
        drawDiamond(ctx, x, y, size, 3*size/2, color);
        return;
    }

    throw new Error('Suit must be one of the following [c, s, h, d]');
}

export function drawFrontCard(ctx, x, y, value, suit, size=55) {
    let width = size;
    let height = 3*size/2;
    let suitColor = getSuitColor(suit);

    // Draw card base
    drawCardBase(ctx, x, y, width, height, suitColor);

    // Draw value
    if (value === 'T') value = '10';
    ctx.font = `bold ${0.4*size}px arial`;
    ctx.textAlign = 'start';
    ctx.fillStyle = 'white';
    ctx.fillText(value, x+0.1*size, y+0.41*size);

    // Draw suit
    drawSuit(ctx, x + width/2, y + width/2, width/2, suit, 'white');
}

export function drawBackCard(ctx, x, y, size=55) {
    let width = size;
    let height = 3*size/2;
    let offset = 0.1*size;

    // Draw card base
    drawCardBase(ctx, x, y, width, height, 'rgb(60, 100, 100)');

    ctx.save();
    ctx.strokeStyle = 'rgb(90, 130, 130)';
    roundRect(ctx, x + offset, y + offset, width - 2*offset, height - 2*offset, 5);
    ctx.stroke();
    ctx.restore();

    ctx.save();
    ctx.strokeStyle = 'rgb(20, 80, 80)';
    roundRect(ctx, x + 3*offset, y + 3*offset, width - 6*offset, height - 6*offset, 5);
    ctx.stroke();
    ctx.restore();
}

