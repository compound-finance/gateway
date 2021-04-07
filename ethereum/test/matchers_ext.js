const chai = require("chai");
const BigNumber = require("bignumber.js");

function extend(object) {
  Object.entries(object).forEach(([name, matcher]) => {
    function wrap(fn) {
      return function (right) {
        var left = this._obj;
        let res = fn(left, right);
        if (res.pass) {
          return new chai.Assertion(true).to.be.equal(true);
        } else {
          return new chai.Assertion("").to.be.equal(res.message);
        }
      }
    }
    console.log("Extending", name);
    chai.Assertion.addMethod(name, wrap(matcher));
  });
}

extend({
  toMatchAddress(actual, expected) {
    actual = actual.hasOwnProperty('address') ? actual.address : actual;
    expected = expected.hasOwnProperty('address') ? expected.address : expected;

    return {
      pass: actual.toLowerCase() == expected.toLowerCase(),
      message: () => `expected (${actual}) == (${expected})`
    }
  },

  toEqualNumber(actual, expected) {
    return {
      pass: actual.toString() == expected.toString(),
      message: () => `expected (${actual.toString()}) == (${expected.toString()})`
    }
  }
});

extend({
  greaterThan(actual, expected) {
    return {
      pass: (new BigNumber(actual)).gt(new BigNumber(expected)),
      message: () => `expected ${actual.toString()} to be greater than ${expected.toString()}`
    }
  }
});

extend({
  toRevert(actual, msg='revert') {
    return {
      pass: !!actual['message'] && actual.message === `VM Exception while processing transaction: ${msg}`,
      message: () => `expected revert, got: ${actual && actual.message ? actual : JSON.stringify(actual)}`
    }
  }
});

extend({
  toBeWithinRange(received, floor, ceiling) {
    const pass = received >= floor && received <= ceiling;
    if (pass) {
      return {
        message: () =>
          `expected ${received} not to be within range ${floor} - ${ceiling}`,
        pass: true,
      };
    } else {
      return {
        message: () =>
          `expected ${received} to be within range ${floor} - ${ceiling}`,
        pass: false,
      };
    }
  },
});
