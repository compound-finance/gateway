const fs = require('fs').promises;
const path = require('path');
const http = require('http');
const https = require('https');
const { URL } = require('url');

function getOptions(url) {
  let u = new URL(url);
  return {
    host: u.hostname,
    path: u.pathname + u.search,
    port: u.port || (u.protocol === 'https:' ? 443 : 80),
  };
}

function download(url, path, options = {}, handle = null) {
  let requestOptions = {
    ...getOptions(url),
    ...options,
  };
  return new Promise(async (resolve, reject) => {
    let f = handle || await fs.open(path, 'w');
    let bail = false;
    let callback = (response) => {
      if (response.statusCode === 301 || response.statusCode === 302) {
        let location = response.headers['location'];
        if (!location) {
          reject(new Error(`Redirect response does not include \`Location\` header`));
        } else {
          bail = true; // Nuts to you.
          download(location, path, options, f).then(() => resolve());
        }
      }
      let promises = [];
      response.on('data', async (chunk) => {
        if (!bail) {
          let promise = new Promise((resolve, reject) => {
            Promise.all(promises).then(() => {
              f.write(chunk).then(() => resolve());
            });
          });
          promises.push(promise);
        }
      });
      response.on('end', async () => {
        if (!bail) {
          await Promise.all(promises);
          await f.close()
          resolve(null);
        }
      });
    }
    let req = (options.port === 80 ? http : https).request(requestOptions, callback).end();
    req.on('error', (e) => {
      reject(e);
    });
  });
}

module.exports = {
  download
};
