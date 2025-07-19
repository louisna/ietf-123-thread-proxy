#!/bin/bash

echo "Before adding backup"
sudo ip netns exec client ip mptcp endpoint add 10.1.0.2 dev mptcp-client-22 id 3 subflow backup
echo "After adding backup"

# Limit the reception window.
echo "Before limiting reception window"
sudo ip netns exec client ip route add 10.1.0.1/24 window 100 dev mptcp-client-22
echo "After limiting reception window"