#!/bin/bash
ROOT=$(cd "$(dirname "${0}")/.." && pwd)
SHELLCHECK_URL=https://github.com/koalaman/shellcheck/releases/download/v0.8.0/shellcheck-v0.8.0.linux.x86_64.tar.xz
errors=false

if ! [ -f "$ROOT/target/x86_64-unknown-linux-musl/release/osc-cost" ]; then
    echo 'error: "target/x86_64-unknown-linux-musl/release/osc-cost" not found'
    errors=true
fi

if [ -z "$OSC_ACCESS_KEY" ]; then
    echo 'error: OSC_ACCESS_KEY not set'
    errors=true
fi

if [ -z "$OSC_SECRET_KEY" ]; then
    echo 'error: OSC_SECRET_KEY not set'
    errors=true
fi

if ! shellcheck --help &> /dev/null ; then
    echo "shellcheck not found. Downloading..."
    if ! wget -O "/tmp/shellcheck.tar.xz" "$SHELLCHECK_URL"; then
        echo "error: shellcheck is not installed and cannot download it"
        errors=true
    fi
    cd /tmp/ && tar xvf "shellcheck.tar.xz"
    if ! sudo cp shellcheck-v0.8.0/shellcheck /usr/local/bin/; then
        echo "error: shellcheck is not installed and installation failed to copy binary to /usr/local/bin/"
        errors=true
    fi
fi

if [ "$errors" = true ]; then
    echo "pre-flight check failed, exiting"
    exit 1
fi

echo -n "self testing with shellcheck... "
if shellcheck int-tests/run.sh &> /dev/null; then
    echo "OK"
else
    echo "FAILED"
fi