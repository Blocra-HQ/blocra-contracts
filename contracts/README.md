# 🚀 Starklytics Bounty Contract Deployment

## 📋 Overview

The Starklytics Bounty Contract is a Cairo smart contract that manages bounty creation, participation, and payouts on Starknet. This guide covers deployment to both Sepolia testnet and mainnet.

## 📁 Contract Location

- **Main Contract**: `../src/bounty_contract.cairo`
- **Library**: `../src/lib.cairo`
- **Deployment Script**: `../scripts/deploy-contract.sh`

## 💰 Deployment Costs

| Network | Declaration | Deployment | Total Cost |
|---------|-------------|------------|------------|
| **Sepolia** | FREE | FREE | **$0** |
| **Mainnet** | ~$5-12 | ~$2-7 | **$7-19** |

## 🛠️ Prerequisites

```bash
# Install Scarb (Cairo package manager)
curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | sh

# Install Starkli (Starknet CLI)
curl https://get.starkli.sh | sh
starkliup
```

## 🔧 Setup Wallet

### Create Account
```bash
# Create keystore
starkli signer keystore new ~/.starkli-wallets/deployer/keystore.json

# Create account descriptor
starkli account oz init ~/.starkli-wallets/deployer/account.json
```

### Fund Account

#### For Sepolia (FREE)
```bash
# Get testnet ETH from faucet
curl -X POST https://starknet-faucet.vercel.app/api/faucet \
  -H "Content-Type: application/json" \
  -d '{"address": "YOUR_ADDRESS_HERE"}'
```

#### For Mainnet
- Transfer ~$20 worth of ETH to your account
- Check balance: `starkli balance YOUR_ADDRESS`

## 🚀 Deployment Commands

### Option 1: Sepolia Testnet (Recommended)

```bash
# Build contract
scarb build

# Declare contract
starkli declare target/dev/starklytics_suite_BountyContract.contract_class.json \
  --account ~/.starkli-wallets/deployer/account.json \
  --keystore ~/.starkli-wallets/deployer/keystore.json \
  --network sepolia

# Deploy contract (replace CLASS_HASH with output from declare)
starkli deploy CLASS_HASH \
  --account ~/.starkli-wallets/deployer/account.json \
  --keystore ~/.starkli-wallets/deployer/keystore.json \
  --network sepolia
```

### Option 2: Mainnet Production

```bash
# Build contract
scarb build

# Declare contract
starkli declare target/dev/starklytics_suite_BountyContract.contract_class.json \
  --account ~/.starkli-wallets/deployer/account.json \
  --keystore ~/.starkli-wallets/deployer/keystore.json \
  --network mainnet

# Deploy contract (replace CLASS_HASH with output from declare)
starkli deploy CLASS_HASH \
  --account ~/.starkli-wallets/deployer/account.json \
  --keystore ~/.starkli-wallets/deployer/keystore.json \
  --network mainnet
```

### Option 3: Automated Script

```bash
# For Sepolia
./scripts/deploy-contract.sh --network sepolia

# For Mainnet
./scripts/deploy-contract.sh --network mainnet
```

## 📝 Update Environment

After successful deployment, update your `.env.local`:

```env
# Replace with your deployed contract address
VITE_BOUNTY_CONTRACT_ADDRESS=0x1234567890abcdef...

# Update network if using Sepolia
VITE_STARKNET_RPC_URL=https://starknet-sepolia.reddio.com/rpc/v0_7
```

## 🔍 Contract Functions

| Function | Description | Access |
|----------|-------------|--------|
| `create_bounty` | Create new bounty | Anyone |
| `join_bounty` | Join existing bounty | Participants |
| `submit_solution` | Submit solution | Participants |
| `select_winner` | Choose winner | Creator only |
| `get_bounty` | View bounty details | Anyone |

## 📊 Verification

### Check Deployment
```bash
# Verify contract exists
starkli call CONTRACT_ADDRESS get_bounty_count

# View contract on explorer
# Sepolia: https://sepolia.starkscan.co/contract/CONTRACT_ADDRESS
# Mainnet: https://starkscan.co/contract/CONTRACT_ADDRESS
```

### Test Contract
```bash
# Create test bounty
starkli invoke CONTRACT_ADDRESS create_bounty \
  --account ~/.starkli-wallets/deployer/account.json \
  --keystore ~/.starkli-wallets/deployer/keystore.json \
  "Test Bounty" "Description" 1000000000000000000 1735689600 10
```

## 🐛 Troubleshooting

### Common Issues

**"Insufficient funds"**
- Add more ETH to your account
- Use Sepolia testnet for free deployment

**"Contract already declared"**
- Use existing class hash for deployment
- Check Starkscan for existing declarations

**"Account not found"**
- Deploy account first: `starkli account deploy`
- Fund account before deployment

**"RPC connection failed"**
- Check network connectivity
- Try alternative RPC endpoints

### Debug Commands
```bash
# Check account balance
starkli balance YOUR_ADDRESS --network sepolia

# View account details
starkli account address ~/.starkli-wallets/deployer/account.json

# Test RPC connection
starkli block-number --network sepolia
```

## 📚 Resources

- [Starknet Documentation](https://docs.starknet.io/)
- [Cairo Book](https://book.cairo-lang.org/)
- [Starkli Documentation](https://book.starkli.rs/)
- [Sepolia Faucet](https://starknet-faucet.vercel.app/)
- [Starkscan Explorer](https://starkscan.co/)

## 🎯 Next Steps

1. **Deploy to Sepolia** for testing
2. **Test all functions** thoroughly  
3. **Deploy to mainnet** when ready
4. **Update frontend** with contract address
5. **Monitor transactions** on Starkscan

---

**💡 Tip**: Always test on Sepolia first before mainnet deployment!