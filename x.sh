#!/bin/sh
set -e
PROJECT_DIR="$(dirname $(readlink -f "$0"))"
test -e "$PROJECT_DIR/.env" && source "$PROJECT_DIR/.env"
COMMAND=$1
INSTALLER_DIR="$PROJECT_DIR/build/installer"
PACKAGE="$PROJECT_DIR/build/installer.tar"
EXE_NAME="cli"

rm_build_dir() {
    if [ ! -d "$PROJECT_DIR/build" ]; then
        return
    fi
    local OUTPUT=$(rm -rf "$PROJECT_DIR/build" 2>&1 > /dev/null | head -n 1)
    echo $MAYBE_RM_ERROR
    test -z "$MAYBE_RM_ERROR" && return
    
    if [[ "$MAYBE_RM_ERROR" =~ "Permission denied" ]]; then
        sudo rm -vr "$PROJECT_DIR/build"
    else
        exit 1
    fi
}

build() {
    rm_build_dir
    cargo build
    
    mkdir -p $INSTALLER_DIR
    cp -rv "$PROJECT_DIR/res/"* $INSTALLER_DIR
    
    if [ -d "$PROJECT_DIR/res_overrides" ]; then
        cp -rv "$PROJECT_DIR/res_overrides/"* $INSTALLER_DIR
    fi
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
        "$INSTALLER_DIR/$EXE_NAME" $*
    ;;
    remote-run)
        if [[ ! -v REMOTE_HOST ]]; then
            echo "Set remote hostname in REMOTE_HOST variable"
            exit 1
        fi
        if [[ ! -v REMOTE_PASS ]]; then
            echo "Set remote password in REMOTE_PASS variable"
            exit 1
        fi
        build
        shift
        sshpass -p $REMOTE_PASS ssh "root@$REMOTE_HOST" "test -d /root/installer && rm -rv /root/installer || exit 0"
        sshpass -p $REMOTE_PASS scp -r "$INSTALLER_DIR" "root@$REMOTE_HOST:/root"
        sshpass -p $REMOTE_PASS ssh "root@$REMOTE_HOST" "/root/installer/$EXE_NAME $*"
    ;;
    *)
        echo -e "Unknown command \"$COMMAND\"\n"
        echo "Available commands:"
        echo "build"
        echo "archiso"
        echo "run"
        echo "remote-run"
        exit 1
    ;;
esac
