# BitQuan Testnet Manager - Tauri Version

This project has been successfully converted from a React/Vite web application to a Rust + Tauri desktop application while maintaining the original design and functionality.

## Project Structure

```
bitquan-testnet-manager/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   └── main.rs     # Main Tauri application with commands
│   ├── Cargo.toml      # Rust dependencies
│   ├── tauri.conf.json # Tauri configuration
│   └── build.rs        # Build script
├── src/                 # React frontend (unchanged)
├── components/          # React components
├── pages/              # React pages
├── types.ts            # TypeScript types
├── package.json        # Node.js dependencies
└── vite.config.ts      # Vite configuration
```

## Key Changes

### Backend (Rust + Tauri)
- Created Rust backend with Tauri framework
- Implemented data structures matching TypeScript interfaces
- Added Tauri commands for data access:
  - `get_miners()` - Returns mining rig data
  - `get_balances()` - Returns wallet balances
  - `get_rigs()` - Returns rig information
  - `get_transactions()` - Returns transaction history
  - `get_alerts()` - Returns system alerts

### Frontend (React)
- Updated components to use Tauri's `invoke()` function
- Replaced hardcoded data with async calls to Rust backend
- Added loading states for better UX
- Maintained original design and styling

### Configuration
- Updated `package.json` with Tauri scripts and dependencies
- Configured `tauri.conf.json` for desktop application settings
- Updated `vite.config.ts` for Tauri compatibility

## Running the Application

### Development Mode
```bash
npm run tauri:dev
```

### Build for Production
```bash
npm run tauri:build
```

## Dependencies

### Rust Dependencies
- `tauri` - Main Tauri framework
- `serde` - Serialization/deserialization
- `serde_json` - JSON handling
- `tokio` - Async runtime

### Node.js Dependencies
- `@tauri-apps/api` - Tauri frontend API
- `react` - UI framework
- `vite` - Build tool

## Features Maintained

✅ Dashboard with balance and miner information  
✅ Rig management with toggle controls  
✅ Wallet functionality with transaction history  
✅ Alerts system  
✅ Statistics page  
✅ Settings with theme switching  
✅ Dark/light mode support  
✅ Responsive design  

## Architecture

The application now follows a desktop-first architecture:

1. **Frontend**: React UI running in a webview
2. **Backend**: Rust process handling business logic
3. **Communication**: Tauri's IPC system for secure data exchange
4. **Distribution**: Single executable with embedded web assets

## Benefits of Tauri Conversion

- **Performance**: Rust backend provides better performance for data processing
- **Security**: Desktop sandboxing and secure IPC
- **Distribution**: Single executable, no browser dependency
- **Cross-platform**: Windows, macOS, and Linux support
- **Size**: Smaller application size compared to Electron
- **Resources**: Lower memory and CPU usage

The original design and user experience remain completely intact while gaining the advantages of a native desktop application.