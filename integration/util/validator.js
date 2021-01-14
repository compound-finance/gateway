const os = require('os');
const path = require('path');
const fs = require('fs').promises;
const util = require('util');
const child_process = require('child_process');
const execFile = util.promisify(child_process.execFile);
const { log, error } = require('./log');

let profile = process.env['PROFILE'] ? process.env['PROFILE'] : 'release';
let target = path.join(__dirname, '..', '..', 'target', profile, 'compound-chain');

async function tmpFile(name) {
  folder = await fs.mkdtemp(path.join(os.tmpdir()));
  return path.join(folder, name);
}

function deepMerge(x, y) {
  Object.entries(y).forEach(([key, val]) => {
    if (typeof (x[key]) === 'object' && typeof (val) === 'object' && !Array.isArray(val)) {
      x[key] = deepMerge(x[key], val);
    } else {
      x[key] = val;
    }
  });

  return x;
}
async function buildChainSpec(props = {}, useTemp = true) {
  let tmpChainSpec = useTemp ? await tmpFile('chainSpec.json') : path.join(__dirname, '..', 'chainSpec.json');
  log('Building chain spec from ' + target + ' to temp file ' + tmpChainSpec);
  let { error, stdout: chainSpecJson, stderr } = await execFile(target, ["build-spec", "--disable-default-bootnode", "--chain", "local"], { maxBuffer: 100 * 1024 * 1024 }); // 100MB
  let chainSpec = deepMerge(JSON.parse(chainSpecJson), props);
  await fs.writeFile(tmpChainSpec, JSON.stringify(chainSpec, null, 2), 'utf8');

  return tmpChainSpec;
}

function spawnValidator(args = [], opts = {}) {
  log(`Starting validator node ${target} with args ${JSON.stringify(args)}`)

  let proc = child_process.spawn(target, args, opts);

  proc.stdout.on('data', (data) => {
    log(`stdout: ${data}`);
  });

  proc.stderr.on('data', (data) => {
    error(`stderr: ${data}`);
  });

  proc.on('close', (code) => {
    log(`child process exited with code ${code}`);
  });

  return proc;
}

module.exports = {
  buildChainSpec,
  spawnValidator
};
