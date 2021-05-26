## About

This project was generated from Playground living here:
[https://play.onflow.org/local-project](https://play.onflow.org/local-project)

We have added some basics tests, which you can run using Node, Flow Emulator and Flow JS Testing Framework.

## Installation

### Node _(optional if you have it installed already)_

Please follow instructions on NodeJS download page and install latest version of NodeJS software:
https://nodejs.org/en/download/

### Flow Emulator _(optional if you have it installed already)_

Flow Emulator is bundled with Flow CLI. You can find instructions on how to install it on Flow Docs site:
https://docs.onflow.org/flow-cli/

### Flow JS Testing Framework

All the necessary files will be installed if you run `npm install` inside `test` folder. For documentations on how to use
the framework you can consult package repository [https://github.com/onflow/flow-js-testing](https://github.com/onflow/flow-js-testing)

### Next Step

When all the above tools would be installed you will need to init Flow Emulator with `flow emulator init` command from within
`test` folder. This will create `flow.json` file in the folder, which is necessary for emulator to operate properly.

Then you can proceed running the tests. We encourage you to run "Create Accounts" suit first, followed by "Deployment" and then
transactions and scripts.

### Afterword

Good luck and happy hacking! :)
