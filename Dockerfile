FROM ubuntu:20.04

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y git curl

RUN mkdir -p /code
WORKDIR /code

RUN git clone --depth 1 https://github.com/compound-finance/gateway.git -b hayesgm/stablenet
WORKDIR /code/gateway

RUN scripts/pull_release.sh m16
RUN chmod +x releases/m16/gateway-linux-x86

CMD releases/m16/gateway-linux-x86 \
  --chain chains/stablenet/chain-spec-raw.json \
  --base-path /chain \
  --ws-external \
  --rpc-methods Unsafe \
  --rpc-external \
  --no-mdns \
  --log runtime=trace,pallet_cash=trace,ethereum_client=debug \
  --validator \
  --ws-max-connections 1000 \
  --rpc-cors=all
