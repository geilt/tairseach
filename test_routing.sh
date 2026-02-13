#!/bin/bash
# Test script to verify routing works for both dot and underscore notation

SOCKET="/Users/geilt/.tairseach/tairseach.sock"

echo "Testing auth.status (dot notation, legacy routing):"
echo '{"jsonrpc":"2.0","id":1,"method":"auth.status","params":{}}' | nc -U "$SOCKET" | jq .

echo ""
echo "Testing auth_status (underscore notation, manifest routing):"
echo '{"jsonrpc":"2.0","id":2,"method":"auth_status","params":{}}' | nc -U "$SOCKET" | jq .

echo ""
echo "Testing config.get (dot notation, legacy routing):"
echo '{"jsonrpc":"2.0","id":3,"method":"config.get","params":{"key":"onepassword.default_vault_id"}}' | nc -U "$SOCKET" | jq .

echo ""
echo "Testing config_get (underscore notation, manifest routing):"
echo '{"jsonrpc":"2.0","id":4,"method":"config_get","params":{"key":"onepassword.default_vault_id"}}' | nc -U "$SOCKET" | jq .
