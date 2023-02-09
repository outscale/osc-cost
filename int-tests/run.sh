#!/bin/bash
ROOT=$(cd "$(dirname "${0}")/.." && pwd)
SHELLCHECK_URL=https://github.com/koalaman/shellcheck/releases/download/v0.8.0/shellcheck-v0.8.0.linux.x86_64.tar.xz
errors=false

oc="$ROOT/target/x86_64-unknown-linux-musl/release/osc-cost"
if ! [ -f "$oc" ]; then
    echo "error: \"$oc\" not found"
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

function ok_or_die {
    what="$1"
    cmd="${*:2}"
    echo -n "$what ... "
    if $cmd &> /tmp/output ; then
        echo "OK"
    else
        echo "FAILED"
        echo "$cmd"
        cat /tmp/output
        exit 1
    fi
}

function ko_or_die {
    what="$1"
    cmd="${*:2}"
    echo -n "$what ... "
    if $cmd &> /tmp/output ; then
        echo "FAILED"
        echo "$cmd"
        cat /tmp/output
        exit 1
    else
        echo "OK"
    fi
}

ok_or_die "self testing with shellcheck" shellcheck int-tests/run.sh

# arg checking
ok_or_die "help command" "$oc" --help
ok_or_die "no arg" "$oc"
ko_or_die "bad arg" "$oc" --bla

# output testing with default input source
ok_or_die "write json output" "$oc" --format json --output output.json
ok_or_die "write hour output" "$oc" --format hour --output output.hour
ok_or_die "write month output" "$oc" --format month --output output.month

# format test with api input source
ok_or_die "json format with implicit api input source" "$oc" --format json
ok_or_die "hour format with implicit api input source" "$oc" --format hour
ok_or_die "month format with implicit api input source" "$oc" --format month

ok_or_die "json format with explicit api input source" "$oc" --source api --format json
ok_or_die "hour format with explicit api input source" "$oc" --source api --format hour
ok_or_die "month format with explicit api input source" "$oc" --source api --format month

ok_or_die "aggregate json format with implicit api input source" "$oc" --aggregate --format json
ko_or_die "aggregate hour format with implicit api input source" "$oc" --aggregate --format hour
ko_or_die "aggregate month format with implicit api input source" "$oc" --aggregate --format month

ok_or_die "aggregate json format with explicit api input source" "$oc" --aggregate --source api --format json
ko_or_die "aggregate hour format with explicit api input source" "$oc" --aggregate --source api --format hour
ko_or_die "aggregate month format with explicit api input source" "$oc" --aggregate --source api --format month

# format test with json input source
ok_or_die "json format with implicit json input source" "$oc" --input output.json --format json
ok_or_die "hour format with implicit json input source" "$oc" --input output.json --format hour
ok_or_die "month format with implicit json input source" "$oc" --input output.json --format month

ok_or_die "json format with explicit json input source" "$oc" --input output.json --source json --format json
ok_or_die "hour format with explicit json input source" "$oc" --input output.json --source json --format hour
ok_or_die "month format with explicit json input source" "$oc" --input output.json --source json --format month

ok_or_die "aggregate json format with implicit json input source" "$oc" --aggregate --input output.json --format json
ko_or_die "aggregate hour format with implicit json input source" "$oc" --aggregate --input output.json --format hour
ko_or_die "aggregate month format with implicit json input source" "$oc" --aggregate --input output.json --format month

ok_or_die "aggregate json format with explicit json input source" "$oc" --aggregate --input output.json --source json --format json
ko_or_die "aggregate hour format with explicit json input source" "$oc" --aggregate --input output.json --source json --format hour
ko_or_die "aggregate month format with explicit json input source" "$oc" --aggregate --input output.json --source json --format month

# input should only work with json as input source
ko_or_die "input as hour should be an error" "$oc" --input output.hour
ko_or_die "input as month should be an error" "$oc" --input output.month
ko_or_die "--input with --source api" "$oc" --input somefile --source api

# bad credentials should result of a failure
SK_TMP=$OSC_SECRET_KEY
OSC_SECRET_KEY="BAD_SECRET_KEY"
ko_or_die "bad ak/sk should fail" "$oc"
OSC_SECRET_KEY=$SK_TMP
unset SK_TMP

# json source can only work with --input for now (stdin not supported yet)
ko_or_die "json source need --input in order to work" "$oc" --source json