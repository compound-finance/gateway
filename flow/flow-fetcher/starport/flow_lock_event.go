package starport

import (
	"fmt"

	"github.com/onflow/cadence"
	"github.com/onflow/flow-go-sdk"
)

// pub event Lock(asset: String, recipient: Address?, amount: UFix64)
type FlowLockEvent cadence.Event

func (evt FlowLockEvent) Asset() string {
	return string(evt.Fields[0].(cadence.String))
}

func (evt FlowLockEvent) Recipient() *flow.Address {
	optionalAddress := (evt.Fields[1]).(cadence.Optional)
	if cadenceAddress, ok := optionalAddress.Value.(cadence.Address); ok {
		recipientAddress := flow.BytesToAddress(cadenceAddress.Bytes())
		return &recipientAddress
	}
	return nil
}

func (evt FlowLockEvent) Amount() uint64 {
	// return float64(evt.Fields[2].(cadence.UFix64).ToGoValue().(uint64)) / 1e8 // ufixed 64 have 8 digits of precision
	return evt.Fields[2].(cadence.UFix64).ToGoValue().(uint64)
}

func (evt FlowLockEvent) String() string {
	return fmt.Sprintf("Lock event: asset: %s, recipient: %s, amount: %d",
		evt.Asset(), evt.Recipient(), evt.Amount())
}
