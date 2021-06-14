package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net/http"
	"os"

	"google.golang.org/grpc"

	"github.com/onflow/flow-go-sdk/client"
)

type FlowEventsInfo struct {
	Topic       string
	StartHeight uint64
	EndHeight   uint64
}

type FlowBlockInfo struct {
	Id     string
	Height uint64
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

		// Try to decode the request body into FlowEventsInfo the struct.
		var eventsInfo FlowEventsInfo
		err := decodeJSONBody(w, r, &eventsInfo)
		if err != nil {
			var mr *malformedRequest
			if errors.As(err, &mr) {
				http.Error(w, mr.msg, mr.status)
			} else {
				log.Println(err.Error())
				http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
			}
			return
		}

		// Fetch Lock events
		events, err := getLockEvents(flowClient, eventsInfo)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		js, err := json.Marshal(events)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.Write(js)
	}
}

func BlockHandler(flowClient *client.Client) func(http.ResponseWriter, *http.Request) {
	if flowClient == nil {
		panic("Flow client is not set")
	}

	return func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/block" {
			http.Error(w, "404 not found.", http.StatusNotFound)
			return
		}

		if r.Method != "GET" {
			http.Error(w, "Method is not supported.", http.StatusNotFound)
			return
		}

		// Try to decode the request body into FlowEventsInfo the struct.
		var blockInfo FlowBlockInfo
		err := decodeJSONBody(w, r, &blockInfo)
		if err != nil {
			var mr *malformedRequest
			if errors.As(err, &mr) {
				http.Error(w, mr.msg, mr.status)
			} else {
				log.Println(err.Error())
				http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
			}
			return
		}

		// Fetch Block info
		events, err := getBlock(flowClient, blockInfo)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		js, err := json.Marshal(events)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.Write(js)
	}
}

func LatestBlockHandler(flowClient *client.Client) func(http.ResponseWriter, *http.Request) {
	if flowClient == nil {
		panic("Flow client is not set")
	}

	return func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/latest_block" {
			http.Error(w, "404 not found.", http.StatusNotFound)
			return
		}

		if r.Method != "GET" {
			http.Error(w, "Method is not supported.", http.StatusNotFound)
			return
		}

		// Fetch Latest sealed Flow block
		latestBlock, err := getLatestBlock(flowClient)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		js, err := json.Marshal(latestBlock)
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

func handleErr(err error) {
	if err != nil {
		panic(err)
	}
}

// 34944396, 34944396, "A.c8873a26b148ed14.Starport.Lock"
func main() {
	// connect to Flow testnet
	flowAccessURL := getEnv("FLOW_ACCESS_URL", "access.devnet.nodes.onflow.org:9000")
	flowClient, err := client.New(flowAccessURL, grpc.WithInsecure())
	handleErr(err)
	err = flowClient.Ping(context.Background())
	handleErr(err)

	// Add `Lock` and other Flow events handler
	http.HandleFunc("/events", EventsHandler(flowClient))

	// Add block handler
	http.HandleFunc("/block", BlockHandler(flowClient))

	// Add latest block handler
	http.HandleFunc("/latest_block", LatestBlockHandler(flowClient))

	// Start the server
	flowServerPort := getEnv("FLOW_SERVER_PORT", "8089")
	fmt.Printf("Starting server at port %s\n", flowServerPort)
	if err := http.ListenAndServe(":"+flowServerPort, nil); err != nil {
		log.Fatal(err)
	}
}
