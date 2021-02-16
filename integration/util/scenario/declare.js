const chalk = require('chalk');

const colors = [
  chalk.green,
  chalk.yellow,
  chalk.blue,
  chalk.magenta,
  chalk.cyan,
  chalk.redBright,
  chalk.greenBright,
  chalk.yellowBright,
  chalk.blueBright,
  chalk.magentaBright,
  chalk.cyanBright,
];

function colorize(id, msg) {
  let colorFn = colors[id % colors.length];
  return colorFn(msg);
}

async function declare(ctx, declareInfo, verb, info, fn) {
  let {
    name,
    id,
    colorId,
    active,
    past,
    failed,
  } = declareInfo;

  let preamble = `[${name}]{action=${id}}: `;

  let log = (x) => {
    ctx.log(colorize(colorId, preamble + x));
  }

  let partsRaw = Array.isArray(info) ? info : [ info ];
  let parts = partsRaw.map((part) => {
    return typeof(part.show) === 'function' ? part.show() : part;
  }).join(' ');

  let showMsg = (tense) => {
    let showVerb = tense === failed ? chalk.red(verb) : verb;
    log(`${tense} ${showVerb} ${parts}`);
  };

  showMsg(active);

  try {
    let res = await fn();

    showMsg(past);

    return res;
  } catch(e) {
    showMsg(failed);
    throw e;
  }
}

module.exports = {
  declare
};
