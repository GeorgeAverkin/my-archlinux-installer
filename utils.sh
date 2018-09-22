#!/usr/bin/zsh

persist() {
  $* && return
  echo "Retry? [Y/n]"
  read answer

  if [[ $answer == "n" ]]; then
    exit 1
  fi
  persist $*
}

askpass() {
  local password1
  local password2

  while : ; do
    echo ${2:-"Enter a password"}
    read -s password1
    echo ${3:-"Repeat the password"}
    read -s password2

    if [ "$password1" = "$password2" ]; then
      break
    fi
    echo ${4:-"Passwords do not match\n"}
  done

  export $1=$password1
}