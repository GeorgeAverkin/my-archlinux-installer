#!/bin/sh
. ./config.sh
DATE=$(date "+%F_%H-%M")
FILE_NAME="$DATE.tar"
EXCLUDE_ARG=""

for path in "${EXCLUDED_PATHS[@]}"; do
    EXCLUDE_ARG="$EXCLUDE_ARG --exclude=$path"
done

tar \
$EXCLUDE_ARG \
--acls \
--xattrs \
--preserve-permissions \
--verbose \
--create \
--file=$FILE_NAME \
/
