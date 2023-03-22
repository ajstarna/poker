import React, { useRef, useEffect, useState, useCallback } from "react";
import { drawTable, drawBackground } from "./draw/drawTable";
import { Player } from "./draw/player";
import { PlayerCard } from "./draw/playerCard";
import { drawFrontCard } from "./draw/drawCard";
import { drawChips } from "./draw/drawChips";

const TableCanvas = props => {
    const canvasRef = useRef(null);
    const [context, setContext] = useState(null);

    const renderFrame = useCallback((gameState, frameCount) => {
        console.log(`renderFrame: ${frameCount}`);
        const canvas = canvasRef.current;
        if (canvas === null) return;

        const context = canvas.getContext('2d');
        const canvasW = canvas.parentNode.getBoundingClientRect().width;
        const canvasH = canvas.parentNode.getBoundingClientRect().height;

        canvas.width = canvasW;
        canvas.height = canvasH;

        // Draw Backgroud
        drawBackground(context, canvasW, canvasH);

        // Draw Table
        drawTable(context, canvasW, canvasH);

        if (gameState) {
            let isShowdown = gameState.street === "showdown" && "showdown" in gameState;
            let bestHand = [];
            let bestHandResult = "";
            let whoShowed = [];
            let numberShowed = 0;
            let showdownEnded = false;

            if (isShowdown) {
                numberShowed = Math.ceil(frameCount / 15);
                showdownEnded = gameState.showdown.length <= numberShowed;
                whoShowed = gameState.showdown.slice(0, numberShowed);
                let winners = whoShowed.filter((showdown) => { return showdown.winner });

                // We have a winner
                if (winners.length > 0) {
                    let winner = winners[0];
                    bestHandResult = winner.hand_result;
                    bestHand = [];
                    bestHand = bestHand.concat(winner.constituent_cards.split("-"));
                    bestHand = bestHand.concat(winner.kickers.split("-"));
                } else if (whoShowed.length > 0) {
                    for (let i = whoShowed.length - 1; i >= 0; i--) {
                        let winner = whoShowed[i];
                        if ("constituent_cards" in winner && "kickers" in winner) {
                            bestHandResult = winner.hand_result;
                            bestHand = [];
                            bestHand = bestHand.concat(winner.constituent_cards.split("-"));
                            bestHand = bestHand.concat(winner.kickers.split("-"));
                        }
                    }
                }
            }

            // Draw players
            for (let i = 0; i < gameState.max_players; i++) {
                let playerState = gameState.players[i];

                if (playerState === null) {
                    continue;
                }

                // the mapped index is always relative to the main player being at index 0
                let mapped_index = (playerState.index - gameState.your_index + 9) % 9;

                // Draw player
                let is_players_turn_to_act = gameState.index_to_act === playerState.index;

                let street_contributions = 0;

                if (gameState.street === "preflop") {
                    street_contributions = playerState.preflop_cont;
                } else if (gameState.street === "flop") {
                    street_contributions = playerState.flop_cont;
                } else if (gameState.street === "turn") {
                    street_contributions = playerState.turn_cont;
                } else if (gameState.street === "river") {
                    street_contributions = playerState.river_cont;
                }

                let player = new Player({
                    index: mapped_index,
                    name: playerState.player_name,
                    money: playerState.money,
                    action: playerState.last_action,
                    is_players_turn_to_act: is_players_turn_to_act,
                    street_contributions: street_contributions,
                    is_active: playerState.is_active
                });

                if (isShowdown) {
                    let showdownPlayers = whoShowed.filter((showdownPlayer) => { return showdownPlayer.index === playerState.index });
                    if (showdownPlayers.length > 0) {
                        let showdownPlayer = showdownPlayers[0];

                        if (showdownEnded) {
                            if (showdownPlayer.winner) {
                                player.won();
                            }

                            if ("payout" in showdownPlayer) {
                                player.street_contributions = showdownPlayer.payout;
                            }
                        }

                        if (showdownPlayer.showCards) {
                            let holeCards = showdownPlayer.hole_cards;

                            let chars = holeCards.split("");

                            player.giveCards(
                                new PlayerCard(true, chars[0], chars[1]),
                                new PlayerCard(true, chars[2], chars[3])
                            );
                        } else {
                            player.muckCards();
                        }
                    } else {
                        player.giveCards(
                            new PlayerCard(false),
                            new PlayerCard(false)
                        );
                    }
                } else {
                    // If this is the player then show the player their cards
                    if ("hole_cards" in gameState &&
                        gameState.hole_cards !== null &&
                        playerState.index === gameState.your_index) {
                        let holeCards = gameState.hole_cards;

                        let chars = holeCards.split("");

                        player.giveCards(
                            new PlayerCard(true, chars[0], chars[1]),
                            new PlayerCard(true, chars[2], chars[3])
                        );
                    } else if (playerState.is_active && playerState.last_action !== "fold") {
                        player.giveCards(
                            new PlayerCard(false),
                            new PlayerCard(false)
                        );
                    }
                }

                player.draw(context, canvasW, canvasH, frameCount);
                player.drawPlayerChips(context, canvasW, canvasH);

                if (gameState.button_idx === playerState.index) {
                    // Draw Button
                    player.drawButton(context, canvasW, canvasH);
                }
            }

            var size = Math.min(canvasW, canvasH);
            let card_size = 0.06 * size;
            let card_offset_y = 0.05 * size;
            let card_margin = card_size / 4;
            let card_start = canvasW / 2 - (6 * card_margin + 5 * card_size) / 2;

            // table cards
            if ("flop" in gameState) {
                const chars = gameState.flop.split("");

                let flopOffset0 = 0;
                let flopOffset1 = 0;
                let flopOffset2 = 0;

                let flopDarken0 = false;
                let flopDarken1 = false;
                let flopDarken2 = false;

                if (isShowdown) {
                    flopDarken0 = true;
                    flopDarken1 = true;
                    flopDarken2 = true;
                }

                if (bestHand.includes(chars[0] + chars[1])) {
                    flopOffset0 = -card_size;
                    flopDarken0 = false;
                }

                if (bestHand.includes(chars[2] + chars[3])) {
                    flopOffset1 = -card_size;
                    flopDarken1 = false;
                }

                if (bestHand.includes(chars[4] + chars[5])) {
                    flopOffset2 = -card_size;
                    flopDarken2 = false;
                }

                drawFrontCard(
                    context,
                    card_start + card_margin,
                    canvasH / 2 - card_size - card_offset_y + flopOffset0,
                    chars[0],
                    chars[1],
                    card_size,
                    flopDarken0
                );
                drawFrontCard(
                    context,
                    card_start + 2 * card_margin + card_size,
                    canvasH / 2 - card_size - card_offset_y + flopOffset1,
                    chars[2],
                    chars[3],
                    card_size,
                    flopDarken1
                );
                drawFrontCard(
                    context,
                    card_start + 3 * card_margin + 2 * card_size,
                    canvasH / 2 - card_size - card_offset_y + flopOffset2,
                    chars[4],
                    chars[5],
                    card_size,
                    flopDarken2
                );
            }
            if ("turn" in gameState) {
                const chars = gameState.turn.split("");

                let turnOffset = 0;
                let turnDarken = false;

                if (isShowdown) {
                    turnDarken = true;
                }

                if (bestHand.includes(chars[0] + chars[1])) {
                    turnOffset = -card_size;
                    turnDarken = false;
                }

                drawFrontCard(
                    context,
                    card_start + 4 * card_margin + 3 * card_size,
                    canvasH / 2 - card_size - card_offset_y + turnOffset,
                    chars[0],
                    chars[1],
                    card_size,
                    turnDarken
                );
            }
            if ("river" in gameState) {
                const chars = gameState.river.split("");

                let riverOffset = 0;
                let riverDarken = false;

                if (isShowdown) {
                    riverDarken = true;
                }

                if (bestHand.includes(chars[0] + chars[1])) {
                    riverOffset = -card_size;
                    riverDarken = false;
                }

                drawFrontCard(
                    context,
                    card_start + 5 * card_margin + 4 * card_size,
                    canvasH / 2 - card_size - card_offset_y + riverOffset,
                    chars[0],
                    chars[1],
                    card_size,
                    riverDarken
                );
            }

            // Draw pots
            if ("pots" in gameState) {
                let pots = gameState.pots;
                pots = pots.filter(pot => pot > 0);

                let size = 0.1 * Math.min(canvasW, canvasH);

                let x = canvasW / 2;
                let y = canvasH / 2 + 0.4 * size;
                let w = size;
                let h = 0.25 * size;

                let total = pots.reduce((partialSum, a) => partialSum + a, 0);
                if (pots.length > 1) {
                    drawChips(
                        context,
                        total,
                        x - 0.9 * size, y, w, h, size
                    );

                    let side = pots.slice(-1);
                    drawChips(
                        context,
                        side,
                        x + 0.9 * size, y, w, h, size
                    );
                } else {
                    drawChips(
                        context,
                        total,
                        x, y, w, h, size
                    );
                }
            }

            // Draw Hand Result
            if (isShowdown) {
                context.font = `bold ${0.025 * size}px arial`;
                context.textAlign = "center";
                context.fillText(bestHandResult, canvasW / 2, canvasH / 2 + 0.15 * size);

            }
        }
    }, [])

    useEffect(() => {
        //i.e. value other than null or undefined
        if (canvasRef.current) {
            const canvas = canvasRef.current;
            const ctx = canvas.getContext("2d");
            setContext(ctx);
        }
    }, []);

    useEffect(() => {
        const frameRate = 200;
        let animationFrameId;
        let lastFrameTime = null;
        let frameCount = 0;

        // Check if null context has been replaced on component mount
        if (context) {
            //Our draw came here
            const render = (time) => {
                if (lastFrameTime !== null) {
                    const delta = time - lastFrameTime;
                    if (delta > frameRate) {
                        renderFrame(props.gameState, frameCount);
                        frameCount++;
                        lastFrameTime = time;
                    }
                } else {
                    renderFrame(props.gameState, frameCount);
                    frameCount++;
                    lastFrameTime = time;
                }

                animationFrameId = window.requestAnimationFrame(render);
            };

            const resize = () => {
                renderFrame(props.gameState, frameCount);
            }

            // Add event listener
            window.addEventListener("resize", resize);

            animationFrameId = window.requestAnimationFrame(render);
        }
        return () => {
            window.cancelAnimationFrame(animationFrameId);
        };
    }, [renderFrame, context, props.gameState]);

    return <canvas ref={canvasRef} className={props.className} />
};

export default TableCanvas;
