#!/bin/bash

set -e

fname=$(dirname "$0")/fake-notifier-status.txt

if [ -f "$fname" ]; then
    retry=$(<"$fname")

    if [ "$retry" -le 0 ]; then
        rm "$fname"
        exec rev
    else
        new_retry=$(( retry - 1 ))
        echo "Error: retry is $retry. Setting it to $new_retry" >&2
        echo "$new_retry" > "$fname"
        exit 1
    fi
else
    read -rN1

    case $REPLY in
        A|a) pattern='A|a'; retry='0';;
        E|e) pattern='E|e'; retry='1';;
        I|i) pattern='I|i'; retry='2';;
        O|o) pattern='O|o'; retry='3';;
        U|u) pattern='U|u'; retry='4';;
    esac

    if [ -n "$pattern" ]; then
        echo "Error: found pattern $pattern. Setting retry to $retry" >&2
        echo "$retry" > "$fname"
        exit 1
    else
        { echo -n "$REPLY"; cat; } | rev
    fi
fi
