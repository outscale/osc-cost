#!/bin/bash
# SPDX-FileCopyrightText: 2023 Outscale SAS
# SPDX-License-Identifier: BSD-3-Clause
set -o errexit
set -o nounset
set -o pipefail

echo "Select Port: $1"
echo "Select Command: $2 $3"
echo "Select Credentials Path: $4"
re_number='^[0-9]+$'
if ! [[ $1 =~ $re_number ]]; then
   echo "$1 is not a port number"
   exit 1
fi

if [[ ! -f "$2" ]]; then
    echo "$2 not found" >&2
    exit 1
fi
jq -n '{default: {"access_key": "$OSC_ACCESS_KEY", "secret_key": "$OSC_SECRET_KEY", "host": "outscale.com", "https": true, "method": "POST", "region": "$OSC_REGION"}}' > $4
while true;
  do 
    echo -e "HTTP/1.1 200 OK\n\n$($2 $3)" \
  | nc -l -k -p $1 -q 1;
done
