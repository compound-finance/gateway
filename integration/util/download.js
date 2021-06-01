const fs = require('fs').promises;
const path = require('path');
const http = require('http');
const https = require('https');
const { URL } = require('url');
const { getOptions } = require('./net');

function download(url, path, options = {}, handle = null) {
  let requestOptions = {
    ...getOptions(url),
    ...options,
  };
  return new Promise(async (resolve, reject) => {
    let bail = false;
    let f_;
    let writeF = async (chunk) => {
      if (!f_) {
        f_ = handle ? await handle : await fs.open(path, 'w');
      }

      await f_.write(chunk);
    }

    let closeF = async () => {
      if (f_) {
        await f_.close();
      }
    }

    let err = async (error) => {
      bail = true;
      await closeF();
      reject(error);
    };

    let callback = (response) => {
      if (response.statusCode === 301 || response.statusCode === 302) {
        let location = response.headers['location'];
        if (!location) {
          err(new Error(`Redirect response does not include \`Location\` header`))
        } else {
          bail = true; // Nuts to you.
          download(location, path, options, f_).then(() => resolve());
        }
      } else if (response.statusCode !== 200) {
        err(new Error(`Server Response: ${JSON.stringify(response.statusCode)} retreiving ${url}`));
      }
      let promises = [];
      response.on('data', async (chunk) => {
        if (!bail) {
          let promise = new Promise((resolve, reject) => {
            Promise.all(promises).then(() => {
              writeF(chunk).then(() => resolve());
            });
          });
          promises.push(promise);
        }
      });
      response.on('end', async () => {
        if (!bail) {
          await Promise.all(promises);
          await closeF();
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
