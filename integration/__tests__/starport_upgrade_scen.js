const { buildScenarios } = require('../util/scenario');

buildScenarios('Starport Upgrade Scenarios', { validators: [], tokens: [] }, [
  {
    name: 'Upgrade Starport to StarportHarness without Initialize',
    scenario: async ({ cashToken, starport, eth, log }) => {
      let starportAddress = starport.starport._address;
      let starportImpl =
        await eth.__deploy(
          'StarportHarness',
          [
            cashToken.ethAddress(),
            eth.accounts[1]
          ]);

      await starport.upgrade(starportImpl);
      expect(starport.starport._address).toMatchAddress(starportAddress); // Didn't change
      expect(await eth.proxyRead(starport.starport, 'implementation')).toMatchAddress(starportImpl._address);
      expect(await eth.proxyRead(starport.starport, 'admin')).toMatchAddress(starport.proxyAdmin._address);
      expect(await starport.starport.methods.admin().call()).toMatchAddress(eth.accounts[1]);
      expect(await starport.starport.methods.counter().call()).toEqual("0");
    }
  },
  {
    name: 'Upgrade Starport to StarportHarness2 with Initialize',
    scenario: async ({ cashToken, starport, eth }) => {
      let starportAddress = starport.starport._address;
      let starportImpl =
        await eth.__deploy(
          'StarportHarness2',
          [
            cashToken.ethAddress(),
            eth.accounts[1]
          ],
        );

      await starport.upgrade(starportImpl, starportImpl.methods.initialize_(10).encodeABI());
      expect(starport.starport._address).toMatchAddress(starportAddress); // Didn't change
      expect(await eth.proxyRead(starport.starport, 'implementation')).toMatchAddress(starportImpl._address);
      expect(await eth.proxyRead(starport.starport, 'admin')).toMatchAddress(starport.proxyAdmin._address);
      expect(await starport.starport.methods.admin().call()).toMatchAddress(eth.accounts[1]);
      expect(await starport.starport.methods.counter().call()).toEqual("10");
    }
  }
]);
