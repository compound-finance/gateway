// File: events.go
package main

import (
	"context"
	"fmt"

	"github.com/onflow/flow-go-sdk/client"
	"github.com/toni/flow-fetcher/starport"
)

type FlowEvent struct {
	BlockId          string `json:"blockId"`
	BlockHeight      uint64 `json:"blockHeight"`
	TransactionId    string `json:"transactionId"`
	TransactionIndex int    `json:"transactionIndex"`
	EventIndex       int    `json:"eventIndex"`
	Topic            string `json:"topic"`
	Data             string `json:"data"`
}

// type LockData struct {
// 	Asset     string  `json:"asset"`
// 	Recipient string  `json:"recipient"`
// 	Amount    float64 `json:"amount"`
// }

func getLockEvents(flowClient *client.Client, eventsInfo FlowEventsInfo) ([]FlowEvent, error) {
	// fetch block events of Starport for the specified range of blocks
	var events []FlowEvent
	blockEvents, err := flowClient.GetEventsForHeightRange(context.Background(), client.EventRangeQuery{
		Type:        eventsInfo.Topic,
		StartHeight: eventsInfo.StartHeight,
		EndHeight:   eventsInfo.EndHeight,
	})
	if err != nil {
		return events, err
	}
	fmt.Println("Block events: ", blockEvents)

	for _, blockEvent := range blockEvents {
		for _, lockEvent := range blockEvent.Events {
			// loop through the Starport.Lock events in this blockEvent
			event := starport.FlowLockEvent(lockEvent.Value)
			fmt.Println("Lock event = ", event)
			fmt.Printf("transactionID: %s, block height: %d\n",
				lockEvent.TransactionID.String(), blockEvent.Height)

			// Build Lock event result data
			var eventRes = FlowEvent{
				BlockId:          blockEvent.BlockID.String(),
				BlockHeight:      blockEvent.Height,
				TransactionId:    lockEvent.TransactionID.String(),
				TransactionIndex: lockEvent.TransactionIndex,
				EventIndex:       lockEvent.EventIndex,
				Topic:            lockEvent.Type,
				Data:             fmt.Sprintf("{\"asset\":\"%s\",\"recipient\":\"%s\",\"amount\":%d}", event.Asset(), event.Recipient(), event.Amount()),
			}

			// // Lock specific fields
			// Asset:     event.Asset(),
			// Recipient: event.Recipient().String(),
			// Amount:    event.Amount(),
			// }
			events = append(events, eventRes)
		}
	}

	return events, nil
}
