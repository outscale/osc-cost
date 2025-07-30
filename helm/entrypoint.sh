#!/bin/bash
# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
set -o errexit
set -o nounset
set -o pipefail

echo "Select Port: $1"
echo "Select Command: $3 $4 $OSCCOST_EXTRA_PARAMS"
echo "Select Credentials Path: $2"
re_number='^[0-9]+$'
if ! [[ $1 =~ $re_number ]]; then
   echo "$1 is not a port number"
   exit 1
fi

if [[ ! -f "$3" ]]; then
    echo "$3 not found" >&2
    exit 1
fi
jq -n '{default: {"access_key": "$OSC_ACCESS_KEY", "secret_key": "$OSC_SECRET_KEY", "host": "outscale.com", "https": true, "method": "POST", "region": "$OSC_REGION"}}' > $2
while true;
  do 
    echo -e "HTTP/1.1 200 OK\nContent-Type: text/plain; version=0.0.4; charset=utf-8\n\n$($3 $4 $OSCCOST_EXTRA_PARAMS)" \
  | nc -l -k -p $1 -q 1;
done
