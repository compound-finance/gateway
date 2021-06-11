const { buildScenarios } = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let lock_scen_info = {
    tokens: [
        { token: 'usdc', balances: { ashley: 1000 } },
        { token: 'maticZrx', balances: { darlene: 1000 } }
    ],
    validators: ['alice', 'bob'],
    actors: ['ashley', 'darlene'],
    chain_opts: {
        matic: {
            name: 'matic',
            provider: 'ganache', // [env=PROVIDER]
            ganache: {
                opts: {},
                web3_port: null
            },
        },
    },
};

buildScenarios('Matic', lock_scen_info, [
    {
        skip: true, // TODO FIX SCEN
        name: 'Matic',
        scenario: async ({ darlene, maticZrx }) => {
            await darlene.lock(1000, maticZrx);
        }
    },
]);
