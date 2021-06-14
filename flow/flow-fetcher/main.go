package main

import (
	"context"
	"fmt"
	"os"
	"github.com/toni/flow-fetcher/starport"
	"github.com/onflow/flow-go-sdk/client"
	"google.golang.org/grpc"
	"log"
    "net/http"
	"encoding/json"
)

type FlowEventsInfo struct {
    Topic string
    StartHeight uint64
	EndHeight uint64
}

// TODO move general values in a parent struct
type LockEvent struct {
	BlockId string
	BlockHeight uint64
	TransactionId string
	TransactionIndex int
	EventIndex int
	Topic string

	// Lock specific fields
	Asset string
	Recipient string
	Amount float64
}

func getLockEvents(flowClient *client.Client, eventsInfo FlowEventsInfo) []LockEvent {
	// fetch latest block
	// latestBlock, err := flowClient.GetLatestBlock(context.Background(), false)
	// handleErr(err)
	// fmt.Println("current height: ", latestBlock.Height)

	// fetch block events of topshot Market.MomentPurchased events for the past 1000 blocks
	blockEvents, err := flowClient.GetEventsForHeightRange(context.Background(), client.EventRangeQuery{
	        Type: eventsInfo.Topic,
			StartHeight: eventsInfo.StartHeight,
			EndHeight: eventsInfo.EndHeight,
	})
	handleErr(err)
	fmt.Println("Block events: ", blockEvents)

	var events []LockEvent
	for _, blockEvent := range blockEvents {
		for _, lockEvent := range blockEvent.Events {
			// loop through the Starport.Lock events in this blockEvent
			event := starport.FlowLockEvent(lockEvent.Value)
			fmt.Println("Lock event = ", event)
			fmt.Printf("transactionID: %s, block height: %d\n",
			lockEvent.TransactionID.String(), blockEvent.Height)

			// Build Lock event result data
			var eventRes = LockEvent {
				BlockId: blockEvent.BlockID.String(),
	            BlockHeight: blockEvent.Height,
	            TransactionId: lockEvent.TransactionID.String(),
	            TransactionIndex: lockEvent.TransactionIndex,
	            EventIndex: lockEvent.EventIndex,
	            Topic: lockEvent.Type,

	            // Lock specific fields
	            Asset: event.Asset(),
	            Recipient: event.Recipient().String(),
	            Amount: event.Amount(),
			}
			events = append(events, eventRes)
		}
	}

	return events
}

func handleErr(err error) {
	if err != nil {
		panic(err)
	}
}

func EventsHandler(flowClient *client.Client) func(http.ResponseWriter, *http.Request) {
    if flowClient == nil {
        panic("Flow client is not set")
    }

    return func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/events" {
			http.Error(w, "404 not found.", http.StatusNotFound)
			return
		}

		if r.Method != "GET" {
			http.Error(w, "Method is not supported.", http.StatusNotFound)
			return
		}

        // Try to decode the request body into the struct. If there is an error,
        // respond to the client with the error message and a 400 status code.
		var eventsInfo FlowEventsInfo
        err := json.NewDecoder(r.Body).Decode(&eventsInfo)
        if err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
        }

		// Fetch Lock events
		events := getLockEvents(flowClient, eventsInfo)
		js, err := json.Marshal(events)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.Write(js)
    }
}

// getEnv get key environment variable if exist otherwise return defalutValue
func getEnv(key, defaultValue string) string {
    value := os.Getenv(key)
    if len(value) == 0 {
        return defaultValue
    }
    return value
}

// 34944396, 34944396, "A.c8873a26b148ed14.Starport.Lock"
func main() {
	// connect to Flow testnet
	flowAccessURL := getEnv("FLOW_ACCESS_URL", "access.devnet.nodes.onflow.org:9000")
	flowClient, err := client.New(flowAccessURL, grpc.WithInsecure())
	handleErr(err)
	err = flowClient.Ping(context.Background())
	handleErr(err)

	// Add Lock events handler
    http.HandleFunc("/events", EventsHandler(flowClient))

	// Start the server
	flowServerPort := getEnv("FLOW_SERVER_PORT", "8089")
    fmt.Printf("Starting server at port %s\n", flowServerPort)
    if err := http.ListenAndServe(":" + flowServerPort, nil); err != nil {
        log.Fatal(err)
    }
}
