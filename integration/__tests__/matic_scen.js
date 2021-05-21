const { buildScenarios } = require('../util/scenario');

let lock_scen_info = {
    tokens: [
        { token: 'usdc', balances: { ashley: 1000 } }
    ],
    validators: ['alice', 'bob']
};

buildScenarios('Matic', lock_scen_info, [
    {
        name: 'Matic',
        scenario: async ({ ashley, usdc, chain }) => {
            expect(true);
        }
    },
]);
