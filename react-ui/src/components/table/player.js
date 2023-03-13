import { roundRect } from "./drawFunctions";
import { getPlayerPostion, getChipsPostion, getButtonPostion } from "./mappedPositions";

export class Player {
    constructor(
        {
        index,
        name, 
        money, 
        action = null, 
        is_players_turn_to_act = false,
        street_contributions = 0, 
        is_active = true} = {}) {
      this.name = name;
      this.index = index;
      this.money = money;
      this.action = action;
      this.is_players_turn_to_act = is_players_turn_to_act;
      this.street_contributions = street_contributions;
      this.is_active = is_active;
      this.cards = [];
    }

    giveCards(card1, card2) {
        this.cards = [
            card1,
            card2
        ];
    }

    drawChips(ctx, width, height) {
        var size = 0.2 * Math.min(width, height);
        
        let [x, y] = getChipsPostion(this.index, width, height, size);

        let x0 = x;
        let y0 = y;

        if (this.street_contributions > 0) {
            // Draw boarder for text
            ctx.fillStyle = "#00000066";
            ctx.strokeStyle = "black";
            roundRect(ctx, x0, y0-10, 80, 20, 5);
            ctx.stroke();
            ctx.fill();

            // Draw text (street contributions)
            ctx.font = '14px arial';
            ctx.fillStyle = "white";
            ctx.fillText(this.street_contributions, x0+25, y0+6);

            // Draw chips
            ctx.beginPath();
            ctx.fillStyle = "yellow";
            ctx.strokeStyle = "black";
            ctx.arc(x0, y0+2, 10, 0, Math.PI * 2, true);
            ctx.fill();
            ctx.stroke();

            ctx.beginPath();
            ctx.fillStyle = "yellow";
            ctx.strokeStyle = "black";
            ctx.arc(x0, y0, 10, 0, Math.PI * 2, true);
            ctx.fill();
            ctx.stroke();

            ctx.beginPath();
            ctx.fillStyle = "white";
            ctx.strokeStyle = "black";
            ctx.arc(x0, y0, 6, 0, Math.PI * 2, true);
            ctx.fill();
            ctx.stroke();
            
        }
    }

    drawButton(ctx, width, height) {
        var size = 0.2 * Math.min(width, height);

        let btn_position = getButtonPostion(this.index, width, height,size);
        ctx.fillStyle = "white";
        ctx.strokeStyle = "black";
    
        ctx.beginPath();
        ctx.arc(
            btn_position[0],
            btn_position[1] + 2,
            10,
            0,
            Math.PI * 2,
            true
        );
        ctx.fill();
        ctx.stroke();
    
        ctx.beginPath();
        ctx.arc(
            btn_position[0],
            btn_position[1],
            10,
            0,
            Math.PI * 2,
            true
        );
        ctx.fill();
        ctx.stroke();
    }

    draw(ctx, width, height) {
        var size = Math.min(width, height);
        let info_size = 0.2 * size;
        let info_offset = info_size / 2;
        let boarder_size = 5;

        let [x, y] = getPlayerPostion(this.index, width, height, info_size / 2, info_size / 4);

        let info_x0 = x - info_size / 2;
        let info_y0 = y + info_offset - info_size;
        let info_x1 = info_size;
        let info_y1 = info_size / 2;

        let card_size = 3*info_size/8;
        let card_margin = card_size / 4;

        // Draw Cards
        if (this.cards.length === 2 && 
            this.is_active && 
            this.action !== "fold") {
            this.cards[0].draw(ctx, info_x0 + card_margin, info_y0 - info_offset, card_size);
            this.cards[1].draw(ctx, info_x0 + info_size - card_size - card_margin, info_y0 - info_offset, card_size);
        }

        // Draw player boarder
        if (this.is_players_turn_to_act) {
            // Draw green boarder if it is the players turn
            ctx.fillStyle = "#3AC547";
            roundRect(ctx,
                info_x0 - boarder_size,
                info_y0 - boarder_size,
                info_x1 + 2 * boarder_size,
                info_y1 + 2 * boarder_size,
                5);
            ctx.fill();
        }

        ctx.fillStyle = "#202020";
        ctx.strokeStyle = "black";
        roundRect(ctx, info_x0, info_y0, info_x1, info_y1, 5);
        ctx.stroke();
        ctx.fill();

        // Draw name
        if (this.is_active && this.action !== "fold") {
            if (this.is_players_turn_to_act) {
                ctx.fillStyle = "#F19B0E";
            } else {
                ctx.fillStyle = "white";
            }
        } else {
            ctx.fillStyle = "#aaaaaa";
        }

        ctx.font = 'bold 16px arial';
        ctx.textAlign = "center";
        ctx.fillText(this.name, info_x0+info_offset, info_y0 + info_size/6);

        // Draw money
        if (this.is_active && this.action !== "fold") {
            ctx.fillStyle = "#3AC547";
        } else {
            ctx.fillStyle = "#206E28";
        }

        ctx.font = 'bold 16px arial';
        ctx.textAlign = "center";
        ctx.fillText(this.money, info_x0+info_offset, info_y0 + 3*info_size/8);

        if (this.action) {

            let action_fill_color = "#999999";
            let action_stroke_color = "#ffffff";

            if (this.action === "fold") {
                action_fill_color = "#550000";
                action_stroke_color = "#dd0000";
            }

            if (this.action === "check" || this.action === "call") {
                action_fill_color = "#157087";
                action_stroke_color = "#22B6DD";
            }

            if (this.action === "bet") {
                action_fill_color = "#168962";
                action_stroke_color = "#24DB9D";
            }

            // Save the default state
            ctx.save();

            // Draw boarder
            ctx.lineWidth = 3;
            ctx.fillStyle = action_fill_color;
            ctx.strokeStyle = action_stroke_color;
            roundRect(ctx, 
                info_x0+info_offset/3, info_y0 + info_size/2, 
                2*info_size/3, 20, 
                5);
            ctx.stroke(); 
            ctx.fill();

            // Draw text
            ctx.font = 'bold 14px arial';
            ctx.textAlign = "center";
            ctx.fillStyle = action_stroke_color;
            ctx.fillText(this.action, info_x0+info_offset, info_y0 + info_size/2 + 15);
            // Restore the default state
            ctx.restore();
        }
    }
  }