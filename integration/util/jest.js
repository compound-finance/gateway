const substrate = require('./substrate');

let chainEventEqual = (event, eventExpectedArgs) => {
  expect(substrate.getEventData(event)).toEqual(eventExpectedArgs);

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
  toChainEventEqual(received, eventExpectedArgs) {
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
  toHaveEthEvent(received, pallet, eventName, eventExpectedArgs) {
    // TODO
    return {
      pass: false,
      message: () => "not implemented"
    }
  }
});
