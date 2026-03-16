# BlocRA - Stellar Smart Contracts

A high-performance smart contract system for bounty management on Stellar built with Soroban.

## 🚀 Quick Start

### Prerequisites
- **Rust** 1.75+ (for smart contracts)
- **Soroban CLI** (for development and deployment)
- **Stellar Wallet** (Freighter recommended)

### Smart Contract Setup
```bash
# Install Soroban CLI
cargo install soroban-cli

# Initialize project (if not already)
soroban contract init

# Build contracts
soroban contract build

# Test contracts
soroban contract test
```

## 📋 Environment Variables

### Required
```bash
SOROBAN_NETWORK_PASSPHRASE=Test SDF Network ; September 2015  # For testnet
# Or for mainnet: Public Global Stellar Network ; September 2015
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org  # For testnet
SOROBAN_SECRET_KEY=your-secret-key
```

## 🏗️ Architecture

- **Language:** Rust
- **Platform:** Soroban (Stellar smart contracts)
- **Features:** Bounty creation, participation, payouts

## 📊 Features

- ✅ Bounty creation and management
- ✅ Participant registration
- ✅ Reward distribution
- ✅ Secure escrow system

## 🔧 Development

```bash
# Build contracts
soroban contract build

# Run tests
soroban contract test

# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bounty_contract.wasm \
  --source your-secret-key \
  --network testnet
```

## 📦 Deployment

### Testnet
```bash
# Deploy contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bounty_contract.wasm \
  --source your-secret-key \
  --network testnet
```

### Mainnet
```bash
# Deploy contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bounty_contract.wasm \
  --source your-secret-key \
  --network mainnet
```

## 📚 Documentation

- **Soroban Docs:** https://soroban.stellar.org/docs
- **Stellar Developer Docs:** https://developers.stellar.org/

## 🔒 Security

- Secure key management
- Contract invariants
- Audit-ready code

## 📈 Performance

- Low transaction fees
- Fast finality
- Scalable architecture

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

MIT License - See LICENSE file for details

## 🔗 Links

- **Stellar Explorer:** https://stellar.expert/explorer/testnet
- **Soroban Playground:** https://playground.soroban.stellar.org/

## 💡 Support

For issues and questions, please open a GitHub issue.

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