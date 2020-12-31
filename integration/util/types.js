const fs = require('fs').promises;
const path = require('path');

async function loadTypes() {
  return JSON.parse(await fs.readFile(path.join(__dirname, '..', '..', 'types.json')));
}

module.exports = {
  loadTypes
};
