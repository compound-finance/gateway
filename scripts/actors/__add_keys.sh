#!env bash

case "$1" in
  "alice")
    port="9933"
    mnemonic="//Alice"
    aura_key="0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
    gran_key="0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"
    ;;
  "bob")
    port="9934"
    mnemonic="//Bob"
    aura_key="0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"
    gran_key="0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69"
    ;;
  "charlie")
    port="9935"
    mnemonic="//Charlie"
    aura_key="0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22"
    gran_key="0x439660b36c6c03afafca027b910b4fecf99801834c62a5e6006f27d978de234f"
    ;;
  *)
    echo "_add_keys.sh {alice,bob,charlie}"
    exit 1
esac

curl "http://localhost:$port" -H "Content-Type:application/json;charset=utf-8" -d \
  "{
    \"jsonrpc\":\"2.0\",
    \"id\":1,
    \"method\":\"author_insertKey\",
    \"params\": [
      \"aura\",
      \"$mnemonic\",
      \"$aura_key\"
    ]
  }"

curl "http://localhost:$port" -H "Content-Type:application/json;charset=utf-8" -d \
  "{
    \"jsonrpc\":\"2.0\",
    \"id\":1,
    \"method\":\"author_insertKey\",
    \"params\": [
      \"gran\",
      \"$mnemonic\",
      \"$gran_key\"
    ]
  }"
