# Database Recovery & Verification

BitQuan includes comprehensive database recovery and verification features.

## Quick Start

\`\`\`bash
# Verify database
bitquan-node verify-db --path data/chaindata

# With backup
bitquan-node verify-db --path data/chaindata --backup

# Rebuild indices
bitquan-node verify-db --path data/chaindata --rebuild
\`\`\`

## Features

- ✅ Database integrity verification
- ✅ Automatic backup before operations
- ✅ Index rebuilding
- ✅ Chain continuity validation

## See docs/storage/ for full documentation.
