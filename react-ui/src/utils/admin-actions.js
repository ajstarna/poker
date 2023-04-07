export const ADMIN_PREFIX = "!";

const ADMIN_CMDS = [
    new RegExp(`(${ADMIN_PREFIX})(small_blind) (\\d)`),
    new RegExp(`(${ADMIN_PREFIX})(big_blind) (\\d)`),
    new RegExp(`(${ADMIN_PREFIX})(buy_in) (\\d)`),
    new RegExp(`(${ADMIN_PREFIX})(set_password) ([^\\s]*)`),
    new RegExp(`(${ADMIN_PREFIX})(show_password)`),
    new RegExp(`(${ADMIN_PREFIX})(add_bot)`),
    new RegExp(`(${ADMIN_PREFIX})(remove_bot)`),
    new RegExp(`(${ADMIN_PREFIX})(restart)`)
];

const HELP_CMD = new RegExp(`(${ADMIN_PREFIX})(help)`);

export function handleAdminCommands(input) {
    if (!input.startsWith(ADMIN_PREFIX)) throw new Error(`Input must start with ${ADMIN_PREFIX}.`);

    let msg = {};

    for (let admin_regex of ADMIN_CMDS) {
        console.log(admin_regex);
        console.log(admin_regex.test(input));
        if (admin_regex.test(input)) {
            let groups = input.match(admin_regex);
            msg["msg_type"] = "admin_command";
            msg["admin_command"] = groups[2];
            if (groups.length > 2)
                msg[groups[2]] = groups[3];
            break;
        }
    }

    if (HELP_CMD.test(input)) {
        msg["msg_type"] = "help";
    }

    return msg;
}
