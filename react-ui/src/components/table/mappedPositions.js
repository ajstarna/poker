export function getPlayerPostion(index, width, height) {
    // this gives us the position of UI for a given MAPPED index
    // (e.g. the main player always maps to index 0)
    var size = Math.min(width, height);

    var cw = width / 2;
    var ch = height / 2;

    var offset = 0.8 * size / 2;

    switch (index) {
        case 0:
            return [cw, ch + offset]
        case 1:
            return [cw - 2*offset/3, ch + 0.95*offset]
        case 2:
            return [cw - offset, ch + offset/3]
        case 3:
            return [cw - offset, ch - offset/3]
        case 4:
            return [cw - offset/3, ch - 4*offset/5]
        case 5:
            return [cw + offset/3, ch - 4*offset/5]
        case 6:
            return [cw + offset, ch - offset/3]
        case 7:
            return [cw + offset, ch + offset/3]
        case 8:
            return [cw + 2*offset/3, ch + 0.95*offset]
        default:
            console.error(`Invalid index given for getPlayerPostion: ${index}. Needs to be between 0 and 8.`);
            break;
    }

    return [0, 0];
}

export function getChipsPostion(index, width, height) {
    // this gives us the position of UI for a given MAPPED index
    // (e.g. the main player always maps to index 0)
    var size = Math.min(width, height);

    var cw = width / 2;
    var ch = height / 2;

    var offset = 0.45 * size / 2;

    switch (index) {
        case 0:
            return [cw, ch + offset]
        case 1:
            return [cw - 0.9*offset, ch + 0.85*offset]
        case 2:
            return [cw - offset, ch + 0.4*offset]
        case 3:
            return [cw - offset, ch - offset/2]
        case 4:
            return [cw - 0.55*offset, ch - 0.9*offset]
        case 5:
            return [cw + 0.65*offset, ch - 0.9*offset]
        case 6:
            return [cw + offset, ch - offset/2]
        case 7:
            return [cw + offset, ch + 0.4*offset]
        case 8:
            return [cw + 0.9*offset, ch + 0.85*offset]
        default:
            console.error(`Invalid index given for getChipsPostion: ${index}. Needs to be between 0 and 8.`);
            break;
    }

    return [0, 0];
}

export function getButtonPostion(index, width, height) {
    // this gives us the position of UI for a given MAPPED index
    // (e.g. the main player always maps to index 0)
    var size = Math.min(width, height);

    var cw = width / 2;
    var ch = height / 2;

    var offset = 0.55 * size / 2;

    switch (index) {
        case 0:
            return [cw + 0.35*offset, ch + 0.9*offset]
        case 1:
            return [cw - 0.6*offset, ch + 0.9*offset]
        case 2:
            return [cw - offset, ch + 0.5*offset]
        case 3:
            return [cw - 1.05*offset, ch - 0.25*offset]
        case 4:
            return [cw - 0.75*offset, ch - 4*offset/5]
        case 5:
            return [cw + 0.15*offset, ch - 0.85*offset]
        case 6:
            return [cw + 0.85*offset, ch - 0.7*offset]
        case 7:
            return [cw + 1.05*offset, ch + 0.1*offset]
        case 8:
            return [cw + offset, ch + 0.75*offset]
        default:
            console.error(`Invalid index given for getChipsPostion: ${index}. Needs to be between 0 and 8.`);
            break;
    }

    return [0, 0];
}