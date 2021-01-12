from web3 import Web3
import requests

URL = 'https://kovan.infura.io/v3/975c0c48e2ca4649b7b332f310050e27'
ADDR = '0x9326BFA02ADD2366b30bacB125260Af641031331'
ADDR2 = '0xd04647B7CB523bb9f26730E9B6dE1174db7591Ad'
DATA = '0xfeaf968c'

def do_web3():
    web3 = Web3(Web3.HTTPProvider(URL))
    abi = '[{"inputs":[],"name":"decimals","outputs":[{"internalType":"uint8","name":"","type":"uint8"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"description","outputs":[{"internalType":"string","name":"","type":"string"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"uint80","name":"_roundId","type":"uint80"}],"name":"getRoundData","outputs":[{"internalType":"uint80","name":"roundId","type":"uint80"},{"internalType":"int256","name":"answer","type":"int256"},{"internalType":"uint256","name":"startedAt","type":"uint256"},{"internalType":"uint256","name":"updatedAt","type":"uint256"},{"internalType":"uint80","name":"answeredInRound","type":"uint80"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"latestRoundData","outputs":[{"internalType":"uint80","name":"roundId","type":"uint80"},{"internalType":"int256","name":"answer","type":"int256"},{"internalType":"uint256","name":"startedAt","type":"uint256"},{"internalType":"uint256","name":"updatedAt","type":"uint256"},{"internalType":"uint80","name":"answeredInRound","type":"uint80"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"version","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"}]'
    addr = '0x9326BFA02ADD2366b30bacB125260Af641031331'
    contract = web3.eth.contract(address=addr, abi=abi)
    latestData = contract.functions.latestRoundData().call()
    print(latestData)

def do_manual():
    post_body = {
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            {
                "to": ADDR,
                "data": DATA
            },
            "latest"
        ],
        "id": 1
    }
    response = requests.post(URL, json=post_body)

    print(response.text)


if __name__ == '__main__':
    do_manual()