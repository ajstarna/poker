class PlayerCard {
    constructor(showing=false, value='A', suit='h') {
        this.showing = showing;
        this.value = value;
        this.suit = suit;
    }

    draw(ctx, position) {
        let x = position[0];
        let y = position[1];
        if (this.showing) {
            drawFrontCard(ctx, x, y, this.value, this.suit);
        } else {
            drawBackCard(ctx, x, y);
        }
    }
}