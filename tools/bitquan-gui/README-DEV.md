# BitQuan GUI Frontend

A modern, responsive React-based GUI for the BitQuan cryptocurrency mining and wallet management application.

## Features

- **Dashboard**: Real-time mining statistics and balance overview
- **Block Progress**: Visual representation of mining progress
- **Rigs Management**: Monitor and control mining rigs/nodes
- **Wallet**: PQC (Post-Quantum Cryptography) secure wallet interface
- **Statistics**: Detailed mining performance analytics
- **Alerts**: Real-time notifications and alerts
- **Settings**: Network configuration and appearance customization

## Technology Stack

- **Frontend**: React 18 with TypeScript
- **Styling**: Tailwind CSS with dark mode support
- **Build Tool**: Vite
- **Desktop App**: Tauri (Rust backend)
- **Icons**: Custom SVG icon components

## Development Setup

### Prerequisites

- Node.js 16+ 
- npm or yarn
- Rust (for Tauri backend)

### Installation

1. Install dependencies:
```bash
npm install
```

2. Run development server:
```bash
npm run dev
```

3. For Tauri development:
```bash
npm run tauri dev
```

### Available Scripts

- `npm run dev` - Start development server
- `npm run build` - Build for production
- `npm run preview` - Preview production build
- `npm run typecheck` - Run TypeScript type checking
- `npm run tauri dev` - Run Tauri app in development mode
- `npm run tauri build` - Build Tauri app for production

## Project Structure

```
bitquan-gui/
├── components/          # Reusable React components
│   ├── icons/          # SVG icon components
│   ├── Card.tsx        # Card container component
│   └── ToggleSwitch.tsx # Toggle switch component
├── pages/              # Main application pages
│   ├── DashboardPage.tsx
│   ├── HalvingPage.tsx
│   ├── RigsPage.tsx
│   ├── SettingsPage.tsx
│   ├── StatisticsPage.tsx
│   └── AlertsPage.tsx
├── src/
│   ├── api/           # API layer for Tauri commands
│   └── pages/         # Wallet-specific components
├── src-tauri/         # Rust backend code
├── App.tsx            # Main application component
├── types.ts           # TypeScript type definitions
└── index.css          # Global styles and Tailwind imports
```

## Responsive Design

The application is fully responsive and optimized for:
- Mobile devices (320px+)
- Tablets (768px+)
- Desktop screens (1024px+)

Key responsive features:
- Adaptive navigation (horizontal on mobile, vertical on desktop)
- Flexible grid layouts
- Touch-friendly interactions
- Optimized tables with horizontal scrolling

## Dark Mode

The application supports both light and dark themes:
- Toggle via Settings page
- System preference detection
- Persistent theme selection
- Smooth transitions between themes

## UI Components

### Card Component
A versatile container component with:
- Consistent padding and spacing
- Hover effects and shadows
- Dark mode support
- Customizable className prop

### ToggleSwitch Component
Accessible toggle switch with:
- Smooth animations
- Keyboard navigation
- ARIA labels
- Hover and focus states

### Icon Components
Custom SVG icons for:
- Dashboard navigation
- Status indicators
- Action buttons
- All icons are optimized for different sizes

## State Management

The application uses React's built-in state management:
- `useState` for local component state
- `useEffect` for side effects and data fetching
- Context API for global theme state

## API Integration

The frontend communicates with the Rust backend via Tauri's invoke system:
- Type-safe API calls
- Error handling
- Loading states
- Async/await patterns

## Performance Optimizations

- Code splitting with dynamic imports
- Memoized components with React.memo
- Optimized re-renders
- Efficient event handlers
- Lazy loading where appropriate

## Accessibility

- Semantic HTML elements
- ARIA labels and roles
- Keyboard navigation support
- Focus management
- Screen reader compatibility
- High contrast ratios

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile browsers (iOS Safari, Android Chrome)

## Contributing

1. Follow the existing code style
2. Use TypeScript for all new code
3. Ensure responsive design
4. Test on multiple screen sizes
5. Check accessibility
6. Run type checking before commits

## Development Tips

- Use the `dev.sh` script for quick setup
- Run `npm run typecheck` frequently
- Test dark mode changes
- Check mobile responsiveness
- Use React DevTools for debugging
- Test Tauri commands separately