# deployments

One file per network, written by `scripts/deploy.sh <network>`. Don't edit by hand.
The backend reads these via `CONTRACT_DEPLOYMENTS_PATH`.

## Schema

```jsonc
{
  "network": "testnet",              // "testnet" | "mainnet"
  "version": "v0.1.0-3-gabc1234",    // `git describe` of the deployed source
  "deployedAt": "2026-11-03T12:00:00Z",
  "approvalVault": "C...",           // contract id
  "invoiceRegistry": "C...",         // contract id
  "wasmHash": {
    "approvalVault": "<64 hex>",     // uploaded Wasm hash
    "invoiceRegistry": "<64 hex>"
  }
}
```

All fields are `null` until the first deployment to that network.

`mainnet.json` stays empty until an external audit is complete (README §5).
`deploy.sh mainnet` refuses to run without `FINANCE_OS_MAINNET_AUDITED=yes`.
