class Player {
    constructor(
        {
        name, 
        index, 
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

    drawChips(ctx, position) {
        let x = position[0];
        let y = position[1];

        if (this.street_contributions > 0) {
            // Draw boarder for text
            ctx.fillStyle = "#00000066";
            ctx.strokeStyle = "black";
            roundRect(ctx, x, y-10, 80, 20, 5);
            ctx.stroke();
            ctx.fill();

            // Draw text (street contributions)
            ctx.font = '14px arial';
            ctx.fillStyle = "white";
            ctx.fillText(this.street_contributions, x+25, y+6);

            // Draw chips
            ctx.beginPath();
            ctx.fillStyle = "yellow";
            ctx.strokeStyle = "black";
            ctx.arc(x, y+2, 10, 0, Math.PI * 2, true);
            ctx.fill();
            ctx.stroke();

            ctx.beginPath();
            ctx.fillStyle = "yellow";
            ctx.strokeStyle = "black";
            ctx.arc(x, y, 10, 0, Math.PI * 2, true);
            ctx.fill();
            ctx.stroke();

            ctx.beginPath();
            ctx.fillStyle = "white";
            ctx.strokeStyle = "black";
            ctx.arc(x, y, 6, 0, Math.PI * 2, true);
            ctx.fill();
            ctx.stroke();
            
        }
    }

    draw(ctx, position) {
        let x = position[0];
        let y = position[1];

        let info_offset = 80;

        // Draw Cards
        if (this.cards.length == 2 && 
            this.is_active && 
            this.action != "fold") {
            this.cards[0].draw(ctx, [x - 60, y]);
            this.cards[1].draw(ctx, [x + 5, y]);
        }

        // Draw player boarder
        if (this.is_players_turn_to_act) {
            // Draw green boarder if it is the players turn
            ctx.fillStyle = "#3AC547";
            roundRect(ctx, x-84, y+info_offset+7, 168, 68, 5);
            ctx.fill();
        }
        ctx.fillStyle = "#202020";
        ctx.strokeStyle = "black";
        roundRect(ctx, x-80, y+info_offset, 160, 60, 5);
        ctx.stroke();
        ctx.fill();

        // Draw name
        if (this.is_active && this.action != "fold") {
            if (this.is_players_turn) {
                ctx.fillStyle = "#F19B0E";
            } else {
                ctx.fillStyle = "white";
            }
        } else {
            ctx.fillStyle = "#aaaaaa";
        }

        ctx.font = 'bold 18px arial';
        ctx.textAlign = "center";
        ctx.fillText(this.name, x, y+info_offset+25);

        // Draw money
        if (this.is_active && this.action != "fold") {
            ctx.fillStyle = "#3AC547";
        } else {
            ctx.fillStyle = "#206E28";
        }

        ctx.font = 'bold 18px arial';
        ctx.textAlign = "center";
        ctx.fillText(this.money, x, y+info_offset+50);

        if (this.action) {

            let action_fill_color = "#999999";
            let action_stroke_color = "#ffffff";

            if (this.action == "fold") {
                action_fill_color = "#550000";
                action_stroke_color = "#dd0000";
            }

            if (this.action == "check" || this.action == "call") {
                action_fill_color = "#157087";
                action_stroke_color = "#22B6DD";
            }

            if (this.action == "bet") {
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
                x-60, y+info_offset+60, 
                120, 20, 
                5);
            ctx.stroke(); 
            ctx.fill();

            // Draw text
            ctx.font = 'bold 16px arial';
            ctx.textAlign = "center";
            ctx.fillStyle = action_stroke_color;
            ctx.fillText(this.action, x, y+info_offset+75);
            // Restore the default state
            ctx.restore();
        }
    }
  }