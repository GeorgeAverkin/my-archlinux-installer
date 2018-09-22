#!/bin/zsh
# This script requires dev_config.sh file in the TMP_DIR directory.
set -e
BASE_PATH="$(dirname $(readlink -f $0))"
TMP_DIR="$BASE_PATH/tmp"
MNT_DIR="$TMP_DIR/mnt"
REMOTE_INSTALLER_DIR="$TMP_DIR/mnt/root/installer"
CONFIG="$TMP_DIR/dev_config.sh"
OVERLAY="$TMP_DIR/overlay"

check_config() {
    if [[ -f $CONFIG ]]; then
        return
    fi
    echo "Developer utils need a configuration in $TMP_DIR to work"
    echo "Please, copy and initialize dev_config.sh"
    exit 1
}

do_mount() {
    mkdir -p $MNT_DIR
    local address="root@$SSH_HOST:/"
    mountpoint -q $MNT_DIR && return
    echo $SSH_PASSWORD | sshfs -o password_stdin $address $MNT_DIR
}

do_umount() {
    umount $MNT_DIR
}

do_sync() {
    do_mount
    touch $REMOTE_INSTALLER_DIR/_ # rm * fails in empty dir 
    rm -rv $REMOTE_INSTALLER_DIR/*
    rsync --exclude tmp $BASE_PATH/* $REMOTE_INSTALLER_DIR

    if [[ -d $OVERLAY ]]; then
        rsync --exclude tmp $OVERLAY/* $REMOTE_INSTALLER_DIR
    fi
    echo "Sync done"
}

do_login() {
    do_mount
    sshpass -p $SSH_PASSWORD ssh root@$SSH_HOST
}

do_install() {
    do_mount
    sshpass -p $SSH_PASSWORD ssh root@$SSH_HOST /root/installer/install.sh
}

main() {
    check_config
    source "$CONFIG"
    local command=$1
    shift
    
    case $command in
    mount)
        do_mount $*
    ;;
    umount)
        do_umount $*
    ;;
    sync)
        do_sync $*
    ;;
    login)
        do_login $*
    ;;
    install)
        do_install $*
    ;;
    *)
        echo "Unknown command $command"
    ;;
    esac
}

main $*