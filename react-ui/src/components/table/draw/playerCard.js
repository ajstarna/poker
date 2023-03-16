import { drawFrontCard, drawBackCard } from './drawCard';

export class PlayerCard {
    constructor(showing=false, value='A', suit='h') {
        this.showing = showing;
        this.value = value;
        this.suit = suit;
    }

    draw(ctx, x, y, size) {
        if (this.showing) {
            drawFrontCard(ctx, x, y, this.value, this.suit, size);
        } else {
            drawBackCard(ctx, x, y, size);
        }
    }
}