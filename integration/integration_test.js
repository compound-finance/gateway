#!env node

let path = require('path');
let child_process = require('child_process');
const { ApiPromise, WsProvider } = require('@polkadot/api');
let target = path.join(__dirname, '..', 'target', 'debug', 'compound-chain');

async function spawnValidator() {
  let ls = child_process.spawn(target, []);

  ls.stdout.on('data', (data) => {
    console.log(`stdout: ${data}`);
  });

  ls.stderr.on('data', (data) => {
    console.error(`stderr: ${data}`);
  });

  ls.on('close', (code) => {
    console.log(`child process exited with code ${code}`);
  });
}

spawnValidator();
