#!/usr/bin/zsh

set -e

if [[ ! "$USER" = "root" ]]; then
  echo "Failed to build liveCD image: root rights required"
  exit 1
fi

BASE_PATH="$(dirname $(readlink -f "$0"))"
source "$BASE_PATH/config.sh"
PROFILE_PATH="/usr/share/archiso/configs/$ARCHISO_PROFILE"

if [[ ! $ARCHISO_WORKING_DIR ]]; then
  ARCHISO_WORKING_DIR="$BASE_PATH/tmp/live"
fi
INSTALLER_PATH="$ARCHISO_WORKING_DIR/airootfs/root/installer"

mkdir -v $ARCHISO_WORKING_DIR
rsync -rv $PROFILE_PATH/* $ARCHISO_WORKING_DIR
rsync -rv --exclude tmp $BASE_PATH/* $INSTALLER_PATH

for pkg in $PACKAGES_ARCHISO; do
  echo $pkg >> "$ARCHISO_WORKING_DIR/packages.x86_64"
done

$ARCHISO_WORKING_DIR/build.sh -v \
  -w "$ARCHISO_WORKING_DIR/work" \
  -o "$ARCHISO_WORKING_DIR/out"

echo "LiveCD built in $ARCHISO_WORKING_DIR/out"