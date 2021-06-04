const http = require('http');
const { genPort } = require('../util');
const { getOptions, readRequest } = require('../net');

class Proxy {
  constructor(serverHost, serverPort, fallbackUrl, hold, ctx) {
    this.serverHost = serverHost;
    this.serverPort = serverPort;
    this.fallbackUrl = fallbackUrl;
    this.ctx = ctx;
    this.replacements = [];
    let that = this;
    this.hold = hold ? new Promise((resolve, reject) => {
      that.holdResolve = resolve;
    }) : Promise.resolve(null);
  }

  replace(regex, fn) {
    this.replacements.push({regex, fn});
  }

  clear() {
    this.replacements.length = 0;
  }

  clearHold() {
    this.holdResolve(null);
  }

  async start() {
    let that = this;
    await new Promise((resolve, reject) => {
      const requestListener = async function (clientReq, clientRes) {
        let clientBodyP = readRequest(clientReq);

        let options = {
          method: clientReq.method,
          headers: clientReq.headers,
          ...getOptions(that.fallbackUrl)
        };

        let proxy = http.request(options, async function (res) {
          await that.hold; // Hold all requests to let us have time to set-up replacements

          let clientBody = await clientBodyP;

          for (let {regex, fn} of that.replacements) {
            if (regex.test(clientBody)) {
              fn(clientReq, clientRes, clientBody);

              return; // Short-circuit
            }
          }

          clientRes.writeHead(res.statusCode, res.headers);
          res.pipe(clientRes, {
            end: true
          });
        });

        clientReq.pipe(proxy, {
          end: true
        });
      }

      let server = http.createServer(requestListener);
      server.listen(that.serverPort, that.serverHost, () => {
        that.ctx.log(`Proxy server listening at ${that.serverUrl()}`)
        that.server = server;
        resolve();
      });
    });
  }

  async teardown() {
    if (this.server) {
      await new Promise((resolve, reject) => {
        this.server.close(resolve);
      });
    }
  }

  serverUrl() {
    return `http://${this.serverHost}:${this.serverPort}/`;
  }
}

let baseInfo = {
  server_port: null,
  server_host: '127.0.0.1',
  fallback_url: null,
  hold: false,
};

async function buildEthProxy(infoHash, ctx) {
  let info = {
    ...baseInfo,
    ...(infoHash === true ? {} : infoHash)
  };

  let serverHost = info.server_host;
  let serverPort = info.server_port || genPort();
  let fallbackUrl = info.fallback_url || ctx.eth.web3Url;
  let hold = info.hold;

  return new Proxy(serverHost, serverPort, fallbackUrl, hold, ctx);
}

module.exports = {
  Proxy,
  buildEthProxy
};
