#!/bin/sh
set -e
REMOTE_HOST=""
REMOTE_PASS=""
COMMAND=$1
PROJECT_DIR="$(dirname $(readlink -f "$0"))"
INSTALLER_DIR="$PROJECT_DIR/build/installer"
PACKAGE="$PROJECT_DIR/build/installer.tar"
EXE_NAME="start"

build() {
    cargo build
    
    if [ -d "$PROJECT_DIR/build" ]; then
        sudo rm -vr "$PROJECT_DIR/build"
    fi
    mkdir -p $INSTALLER_DIR
    cp -rv "$PROJECT_DIR/res/"* $INSTALLER_DIR
    cp -v "$PROJECT_DIR/target/debug/$EXE_NAME" $INSTALLER_DIR
    
    pushd $INSTALLER_DIR
    tar -cvf $PACKAGE *
    popd
}

case $COMMAND in
    build)
        build
    ;;
    archiso)
        build
        shift
        sudo "$INSTALLER_DIR/$EXE_NAME" archiso --working-dir="$PROJECT_DIR/build/archiso"
    ;;
    run)
        build
        shift
        sudo "$INSTALLER_DIR/$EXE_NAME" $*
    ;;
    remote-run)
        build
        shift
        sshpass -p $REMOTE_PASS ssh "root@$REMOTE_HOST" "test -d /root/installer && rm -rv /root/installer || exit 0"
        sshpass -p $REMOTE_PASS scp -r "$INSTALLER_DIR" "root@$REMOTE_HOST:/root"
        sshpass -p $REMOTE_PASS ssh "root@$REMOTE_HOST" "/root/installer/start $*"
    ;;
    *)
        echo "Unknown command \"$COMMAND\""
        exit 1
    ;;
esac