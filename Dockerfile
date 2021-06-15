
ARG PULL_FROM

# FROM arm64v8/ubuntu:20.04
FROM $PULL_FROM

# need this due to an insane thing that happens when you run `apt update`
ENV TZ=UTC
ENV OSTYPE=linux-gnu

# run apt update in this step to lock in this layer reducing build time
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && apt update

# get a layer for "get_substrate.sh" completed
COPY ./scripts/get_substrate.sh get_substrate.sh

RUN chmod u+x ./get_substrate.sh && \
     ./get_substrate.sh --fast

# layer for get_substrate.sh complete
# build layers
COPY ./ /src

WORKDIR /src

ENV PATH=/root/.cargo/bin:$PATH

# standard build
RUN cargo build --release

# volume mount the target directory in a subfolder for later extraction
VOLUME ./target/docker ./target