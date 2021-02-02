
#!/usr/bin/env bash

curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d \
  '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "aura",
      "//Alice",
      "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
    ]
  }'

curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d \
  '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "gran",
      "//Alice",
      "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"
    ]
  }'

curl http://localhost:9934 -H "Content-Type:application/json;charset=utf-8" -d \
  '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "aura",
      "//Bob",
      "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"
    ]
  }'

curl http://localhost:9934 -H "Content-Type:application/json;charset=utf-8" -d \
  '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "gran",
      "//Bob",
      "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69"
    ]
  }'

curl http://localhost:9935 -H "Content-Type:application/json;charset=utf-8" -d \
  '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "aura",
      "//Charlie",
      "0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22"
    ]
  }'

curl http://localhost:9935 -H "Content-Type:application/json;charset=utf-8" -d \
  '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
      "gran",
      "//Charlie",
      "0x439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f"
    ]
  }'