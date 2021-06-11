const net = require('net');
const util = require('util');

const createConnection = util.promisify(net.createConnection);

async function canConnectTo(host, port, timeout=5000) {
  return new Promise((resolve, reject) => {
    const client = net.createConnection({ host, port, timeout }, () => {
      resolve(true); 
      client.end();
    });
    client.on('error', () => {
      resolve(false);
    });

    client.on('timeout', () => {
      resolve(false);
    });
  });
}

function getOptions(url) {
  let u = new URL(url);
  return {
    host: u.hostname,
    path: u.pathname + u.search,
    port: u.port || (u.protocol === 'https:' ? 443 : 80),
  };
}

async function readRequest(req) {
  return new Promise((resolve, reject) => {
    let data = '';

    req.on('data', chunk => {
      data += chunk;
    });

    req.on('end', () => {
      resolve(data);
    });
  });
}

module.exports = {
  canConnectTo,
  getOptions,
  readRequest
};
