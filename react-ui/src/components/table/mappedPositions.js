export function getPlayerPostion(index, width, height, xOffset, yOffset) {
    // this gives us the position of UI for a given MAPPED index
    // (e.g. the main player always maps to index 0)
    var size = Math.min(width, height);
    var margin = 0.01 * size;

    switch (index) {
        case 0:
            return [width / 2, height - margin - yOffset]
        case 1:
            return [width / 4, height - 4 * margin - yOffset]
        case 2:
            return [xOffset + margin, height - height / 3 + yOffset]
        case 3:
            return [xOffset + margin, height - 2 * height / 3 + yOffset]
        case 4:
            return [width / 3, 4 * margin + 4 * yOffset]
        case 5:
            return [2 * width / 3 - xOffset / 2, 4 * margin + 4 * yOffset]
        case 6:
            return [width - xOffset - margin, height / 3 + yOffset]
        case 7:
            return [width - xOffset - margin, 2 * height / 3 + yOffset]
        case 8:
            return [3 * width / 4, height - 4 * margin - yOffset]
        default:
            console.error(`Invalid index given for getPlayerPostion: ${index}. Needs to be between 0 and 8.`);
            break;
    }

    return [0, 0];
}

export function getChipsPostion(index, width, height, size) {
    // this gives us the position of UI for a given MAPPED index
    // (e.g. the main player always maps to index 0)
    switch (index) {
        case 0:
            return [width / 2 - size/4, height - 3*size/2]
        case 1:
            return [width / 4 + size/4, height - 7*size/4]
        case 2:
            return [5*size/4, height / 2 + size/2]
        case 3:
            return [5*size/4, height / 2 - size/2]
        case 4:
            return [width / 3, 3*size/2]
        case 5:
            return [2 * width / 3 - size / 2,  3*size/2]
        case 6:
            return [width - 7*size/4, height / 2 - size/2]
        case 7:
            return [width - 7*size/4, height / 2 + size/2]
        case 8:
            return [3 * width / 4 - 3*size/4, height - 7*size/4]
        default:
            console.error(`Invalid index given for getChipsPostion: ${index}. Needs to be between 0 and 8.`);
            break;
    }

    return [0, 0];
}

export function getButtonPostion(index, width, height, size) {
    // this gives us the position of UI for a given MAPPED index
    // (e.g. the main player always maps to index 0)
    switch (index) {
        case 0:
            return [width / 2 + size/2, height - 3*size/2 + 20]
        case 1:
            return [width / 4 + 3*size/4, height - 7*size/4 + 40]
        case 2:
            return [5*size/4 + 40, height / 2 + size/2 + 40]
        case 3:
            return [5*size/4, height / 2 - size/2 + 40]
        case 4:
            return [width / 3 - size/3, 3*size/2 + 20]
        case 5:
            return [2 * width / 3 - 3 * size / 4,  3*size/2 - 20]
        case 6:
            return [width - 8*size/5, height / 2 - size/2 - 40]
        case 7:
            return [width - 4*size/3, height / 2 + size/2 - 40]
        case 8:
            return [3 * width / 4, height - 7*size/4]
        default:
            console.error(`Invalid index given for getButtonPostion: ${index}. Needs to be between 0 and 8.`);
            break;
    }

    return [0, 0];
}