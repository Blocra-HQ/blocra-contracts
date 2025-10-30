#!/bin/bash

# Starknet Contract Deployment Script
set -e

echo "🚀 Deploying Bounty Contract to Starknet Mainnet..."

# Check if Scarb is installed
if ! command -v scarb &> /dev/null; then
    echo "❌ Scarb not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh
    source ~/.bashrc
fi

# Check if Starkli is installed
if ! command -v starkli &> /dev/null; then
    echo "❌ Starkli not found. Installing..."
    curl https://get.starkli.sh | sh
    starkliup
fi

# Build the contract
echo "🔨 Building contract..."
scarb build

# Check if contract compiled successfully
if [ ! -f "target/dev/starklytics_suite_BountyContract.contract_class.json" ]; then
    echo "❌ Contract compilation failed"
    exit 1
fi

echo "✅ Contract compiled successfully"

# Declare the contract (replace with actual account details)
echo "📝 Declaring contract..."
CONTRACT_CLASS_HASH=$(starkli declare target/dev/starklytics_suite_BountyContract.contract_class.json \
    --account ~/.starkli-wallets/deployer/account.json \
    --keystore ~/.starkli-wallets/deployer/keystore.json \
    --network mainnet 2>/dev/null | grep -o '0x[0-9a-fA-F]*' | head -1)

if [ -z "$CONTRACT_CLASS_HASH" ]; then
    echo "❌ Contract declaration failed"
    exit 1
fi

echo "✅ Contract declared with class hash: $CONTRACT_CLASS_HASH"

# Deploy the contract
echo "🚀 Deploying contract..."
CONTRACT_ADDRESS=$(starkli deploy $CONTRACT_CLASS_HASH \
    --account ~/.starkli-wallets/deployer/account.json \
    --keystore ~/.starkli-wallets/deployer/keystore.json \
    --network mainnet 2>/dev/null | grep -o '0x[0-9a-fA-F]*' | head -1)

if [ -z "$CONTRACT_ADDRESS" ]; then
    echo "❌ Contract deployment failed"
    exit 1
fi

echo "✅ Contract deployed successfully!"
echo "📍 Contract Address: $CONTRACT_ADDRESS"
echo "🔗 Class Hash: $CONTRACT_CLASS_HASH"

# Save deployment info
cat > deployment.json << EOF
{
  "network": "mainnet",
  "contractAddress": "$CONTRACT_ADDRESS",
  "classHash": "$CONTRACT_CLASS_HASH",
  "deployedAt": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "deployer": "$(starkli account address ~/.starkli-wallets/deployer/account.json)"
}
EOF

echo "💾 Deployment info saved to deployment.json"
echo "🎉 Deployment complete!"