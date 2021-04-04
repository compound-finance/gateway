#!/bin/bash
# Copyright 2015-2020 Parity Technologies (UK) Ltd.

if ! which rustup >/dev/null 2>&1; then
    if [[ "$OSTYPE" == "linux-gnu" ]]; then
	set -e
	if [[ $(whoami) == "root" ]]; then
	    MAKE_ME_ROOT=
	else
	    MAKE_ME_ROOT=sudo
	fi

	if [ -f /etc/redhat-release ]; then
	    echo "Redhat Linux detected."
	    echo "This OS is not supported with this script at present. Sorry."
	    echo "Please refer to https://github.com/paritytech/substrate for setup information."
	    exit 1
	elif [ -f /etc/SuSE-release ]; then
	    echo "Suse Linux detected."
	    echo "This OS is not supported with this script at present. Sorry."
	    echo "Please refer to https://github.com/paritytech/substrate for setup information."
	    exit 1
	elif [ -f /etc/arch-release ]; then
	    echo "Arch Linux detected."
	    $MAKE_ME_ROOT pacman -Syu --needed --noconfirm cmake gcc openssl-1.0 pkgconf git clang
	    export OPENSSL_LIB_DIR="/usr/lib/openssl-1.0";
	    export OPENSSL_INCLUDE_DIR="/usr/include/openssl-1.0"
	elif [ -f /etc/mandrake-release ]; then
	    echo "Mandrake Linux detected."
	    echo "This OS is not supported with this script at present. Sorry."
	    echo "Please refer to https://github.com/paritytech/substrate for setup information."
	    exit 1
	elif [ -f /etc/debian_version ]; then
	    echo "Ubuntu/Debian Linux detected."
	    $MAKE_ME_ROOT apt update
	    $MAKE_ME_ROOT apt install -y cmake pkg-config libssl-dev git gcc build-essential git clang libclang-dev
	else
	    echo "Unknown Linux distribution."
	    echo "This OS is not supported with this script at present. Sorry."
	    echo "Please refer to https://github.com/paritytech/substrate for setup information."
	    exit 1
	fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
	set -e
	echo "Mac OS (Darwin) detected."

	if ! which brew >/dev/null 2>&1; then
	    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"
	fi

	brew update
	brew install openssl cmake llvm
    elif [[ "$OSTYPE" == "freebsd"* ]]; then
	echo "FreeBSD detected."
	echo "This OS is not supported with this script at present. Sorry."
	echo "Please refer to https://github.com/paritytech/substrate for setup information."
	exit 1
    else
	echo "Unknown operating system."
	echo "This OS is not supported with this script at present. Sorry."
	echo "Please refer to https://github.com/paritytech/substrate for setup information."
	exit 1
    fi

    curl https://sh.rustup.rs -sSf | sh -s -- -y
    source ~/.cargo/env
fi

if ! rustup target list | grep 'wasm32-unknown-unknown (installed)' >/dev/null 2>&1; then
    NIGHTLY=nightly-2021-03-24
    rustup toolchain install $NIGHTLY
    rustup update $NIGHTLY
    rustup target add wasm32-unknown-unknown --toolchain $NIGHTLY
    rustup default $NIGHTLY
fi

if [[ "$1" == "--fast" ]]; then
    echo "Skipped cargo install of 'substrate' and 'subkey'"
    echo "You can install manually by cloning the https://github.com/paritytech/substrate repo,"
    echo "and using cargo to install 'substrate' and 'subkey' from the repo path."
else
    g=$(mktemp -d)
    git clone https://github.com/paritytech/substrate "$g"
    pushd "$g"
    cargo install --force --path ./bin/node/cli #substrate
    cargo install --force --path ./bin/utils/subkey subkey
    popd
fi

source ~/.cargo/env
