
#!/usr/bin/env bash
set -e

cd $(dirname ${BASH_SOURCE[0]})
echo $(dirname ${BASH_SOURCE[0]})
# Submit a new key via RPC, connect to where your `rpc-port` is listening
# FILE=$1

subkey inspect --scheme ed25519 "clip organ olive upper oak void inject side suit toilet stick narrow"
# subkey inspect --scheme SR25519 "clip organ olive upper oak void inject side suit toilet stick narrow"

# curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d "@$(pwd)/key_rpc/alice_babe_rpc"
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d "@$(pwd)/key_rpc/alice_gran_rpc"


subkey inspect --scheme ed25519 "paper next author index wedding frost voice mention fetch waste march tilt"
# subkey inspect --scheme SR25519 "paper next author index wedding frost voice mention fetch waste march tilt"

# curl http://localhost:9934 -H "Content-Type:application/json;charset=utf-8" -d "@$(pwd)/key_rpc/bob_babe_rpc"
curl http://localhost:9934 -H "Content-Type:application/json;charset=utf-8" -d "@$(pwd)/key_rpc/bob_gran_rpc"