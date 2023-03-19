export class TableInfo {
    constructor(tableName) {
        this.tableName = tableName;
        this.hasInformation = false;
    }

    setInformation(smallBlind, bigBlind, buyIn, maxPlayers, numHumans, numBots) {
        this.smallBlind = smallBlind;
        this.bigBlind = bigBlind;
        this.buyIn = buyIn;
        this.maxPlayers = maxPlayers;
        this.numHumans = numHumans;
        this.numBots = numBots;

        this.hasInformation = true;
    }
}