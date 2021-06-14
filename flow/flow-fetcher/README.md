Go server for fetching Starport events from Flow

to run:
`go run *.go`

to fetch `Lock` events:
`curl -X GET http://localhost:8089/events -H "Content-type: application/json" -d '{"topic": "A.c8873a26b148ed14.Starport.Lock", "StartHeight": 34944396, "EndHeight": 34944396}'`