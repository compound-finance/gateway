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

module.exports = {
  canConnectTo
};
