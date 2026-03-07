#!/bin/bash

CONTRACT="0xCd45aC05fe7C014D6B2F62b3446E2A91D661a236"
RPC="https://mainnet.base.org"

echo "🔍 Checking contract activity on Base"
echo "Contract: $CONTRACT"
echo ""

# Get latest block
LATEST=$(curl -s -X POST $RPC \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)

LATEST_DEC=$((16#${LATEST:2}))
echo "Latest block: $LATEST_DEC"

# Check if contract exists
echo ""
echo "📝 Checking if contract exists..."
CODE=$(curl -s -X POST $RPC \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$CONTRACT\",\"latest\"],\"id\":1}" | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)

if [ "$CODE" = "0x" ]; then
  echo "❌ Contract does not exist or has no code"
  exit 1
else
  echo "✅ Contract exists (code length: ${#CODE} chars)"
fi

# Check transaction count
echo ""
echo "📊 Checking transaction count..."
TX_COUNT=$(curl -s -X POST $RPC \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getTransactionCount\",\"params\":[\"$CONTRACT\",\"latest\"],\"id\":1}" | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)

TX_COUNT_DEC=$((16#${TX_COUNT:2}))
echo "Outgoing transactions: $TX_COUNT_DEC"

# Try to find when contract was deployed (binary search)
echo ""
echo "🔍 Finding deployment block..."
LOW=0
HIGH=$LATEST_DEC
DEPLOYMENT=0

while [ $LOW -le $HIGH ]; do
  MID=$(( (LOW + HIGH) / 2 ))
  MID_HEX=$(printf "0x%x" $MID)
  
  CODE_AT=$(curl -s -X POST $RPC \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$CONTRACT\",\"$MID_HEX\"],\"id\":1}" | \
    grep -o '"result":"[^"]*"' | cut -d'"' -f4)
  
  if [ "$CODE_AT" != "0x" ] && [ ${#CODE_AT} -gt 4 ]; then
    DEPLOYMENT=$MID
    HIGH=$((MID - 1))
  else
    LOW=$((MID + 1))
  fi
  
  # Limit iterations
  if [ $((HIGH - LOW)) -lt 1000 ]; then
    break
  fi
done

echo "✅ Contract deployed around block: $DEPLOYMENT"

# Check for events in last 10k blocks
echo ""
echo "📡 Checking for events in last 10,000 blocks..."
FROM_DEC=$((LATEST_DEC - 10000))
FROM_HEX=$(printf "0x%x" $FROM_DEC)

EVENTS=$(curl -s -X POST $RPC \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getLogs\",\"params\":[{\"address\":\"$CONTRACT\",\"fromBlock\":\"$FROM_HEX\",\"toBlock\":\"$LATEST\"}],\"id\":1}")

EVENT_COUNT=$(echo "$EVENTS" | grep -o '"result":\[' | wc -l)

if echo "$EVENTS" | grep -q '"result":\[\]'; then
  echo "📝 Events found: 0"
else
  echo "📝 Events found: $(echo "$EVENTS" | grep -o '"blockNumber"' | wc -l)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 SUMMARY:"
echo "   Contract exists: ✅"
echo "   Deployed at block: ~$DEPLOYMENT"
echo "   Outgoing TX count: $TX_COUNT_DEC"
echo "   Events (last 10k blocks): $(echo "$EVENTS" | grep -o '"blockNumber"' | wc -l)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $TX_COUNT_DEC -gt 0 ]; then
  echo "✅ Contract has sent $TX_COUNT_DEC transactions"
  echo "💡 Transactions exist but may not be in recent blocks"
  echo "💡 Need to scan from deployment block ($DEPLOYMENT) to find them"
else
  echo "⚠️  Contract has not sent any transactions"
  echo "💡 This might be a receiver-only contract"
fi
