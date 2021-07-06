// File: blocks.go
package main

import (
	"context"
	"fmt"

	"github.com/onflow/flow-go-sdk"
	"github.com/onflow/flow-go-sdk/client"
)

type Block struct {
	BlockId       string `json:"blockId"`
	ParentBlockId string `json:"parentBlockId"`
	Height        uint64 `json:"height"`
	Timestamp     string `json:"timestamp"`
}

func getLatestBlock(flowClient *client.Client) (Block, error) {
	// Fetching only sealed blocks here
	isSealed := true
	latestBlock, err := flowClient.GetLatestBlock(context.Background(), isSealed)
	if err != nil {
		return Block{}, err
	}

	fmt.Println("Latest block: ", latestBlock)

	var blockRes = Block{
		BlockId:       latestBlock.ID.String(),
		ParentBlockId: latestBlock.ParentID.String(),
		Height:        latestBlock.Height,
		Timestamp:     latestBlock.Timestamp.String(),
	}

	return blockRes, nil
}

func getBlock(flowClient *client.Client, blockInfo FlowBlockInfo) (Block, error) {
	// If height and id of block are no set, return the latest block
	if blockInfo.Id == "" && blockInfo.Height == 0 {
		return getLatestBlock(flowClient)
	}

	block, err := func() (*flow.Block, error) {
		if blockInfo.Height == 0 {
			return flowClient.GetBlockByID(context.Background(), flow.HexToID(blockInfo.Id))
		} else {
			return flowClient.GetBlockByHeight(context.Background(), blockInfo.Height)
		}
	}()

	if err != nil {
		return Block{}, err
	}

	fmt.Printf("Block %+v for data %+v:\n", block, blockInfo)

	blockRes := Block{
		BlockId:       block.ID.String(),
		ParentBlockId: block.ParentID.String(),
		Height:        block.Height,
		Timestamp:     block.Timestamp.String(),
	}

	return blockRes, nil
}
