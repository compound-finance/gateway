const substrate = require('./substrate');

let fail = (msg) => {
  return {
    pass: false,
    message: () => msg
  };
};

let chainEventEqual = (event, eventExpectedArgs) => {
  expect(substrate.getEventData(event)).toMatchObject(eventExpectedArgs);

  return {
    pass: true
  }
};

expect.extend({
  toMatchAddress(actual, expected) {
    return {
      pass: actual.toLowerCase() == expected.toLowerCase(),
      message: () => `expected (${actual}) == (${expected})`
    }
  },

  toHaveReason(received, reason) {
    if (!received) {
      return {
        pass: false,
        message: () => `Expected failure event, got undefined`
      };
    }
    let data = substrate.getEventData(received);

    if (!data['Reason']) {
      return {
        pass: false,
        message: () => `Expected Reason key, got: ${JSON.stringify(data)}`
      };
    } else {
      let receivedReason = data['Reason'];
      if (typeof(reason) === 'string') {
        expect(receivedReason).toEqual({[reason]: null});
      } else {
        expect(receivedReason).toEqual(reason);
      }

      return {
        pass: true
      };
    }
  },
  toMatchChainEvent(received, eventExpectedArgs) {
    return chainEventEqual(received, eventExpectedArgs);
  },
  toHaveChainEvent(received, pallet, eventName, eventExpectedArgs) {
    let event = substrate.findEvent(received, pallet, eventName);

    if (!event) {
      return {
        pass: false,
        message: () => `Expected ${received} to be have event ${pallet}:${eventName}`,
      };
    }

    return chainEventEqual(event, eventExpectedArgs);
  },
  toHaveEthEvent(received, eventName, args) {
    if (!received.events) {
      return fail("Tx does not have `events` key");
    }
    let event = received.events[eventName];
    if (!event) {
      return fail(`Missing event \`${eventName}\` on tx, found: ${JSON.stringify(Object.keys(received.events))}`);
    }
    expect(event.returnValues).toMatchObject(args);
    return {
      pass: true
    };
  },
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
  toEthRevert(actual, reason = null) {
    if (reason === null) {
      return {
        pass: !!actual['message'] && (
          actual.message.startsWith(`Transaction has been reverted by the EVM:`)
          || actual.message.startsWith(`VM Exception while processing transaction: revert`)
        ),
        message: () => `expected revert without reason, got: ${JSON.stringify(actual)}`
      };
    } else {
      return {
        pass: !!actual['reason'] && actual.reason === reason,
        message: () => `expected revert with reason "${reason}", got: ${JSON.stringify(actual)}`
      };
    }
  },
  toEqualSet(actual, expected) {
    let actualSorted = [...actual].sort();
    let expectedSorted = [...expected].sort();
    expect(actualSorted).toEqual(expectedSorted);
    return {
      pass: true
    };
  }
});
