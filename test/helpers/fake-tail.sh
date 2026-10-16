#!/bin/bash

set -e

# Keep in mind that you cannot go too fast here, as there is the
# MIN_RECV_TIME sleep interval to take into account

sleep 0.250

echo 'Hi, this is sample log content'
echo
echo 'Gonna wait some time'
sleep 7.500

echo 'And now guess what? Some more time'
sleep 1.500

echo 'A little more'
sleep 1.000

# echo 'Ok, "stop"'
echo -n 'Ok, "stop"'
