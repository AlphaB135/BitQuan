import React from 'react';
import { useState, useEffect } from 'react';
import DashboardPage from './pages/DashboardPage';
import HalvingPage from './pages/HalvingPage';
import RigsPage from './pages/RigsPage';
import { WalletPage } from './src/pages/WalletPage';
import StatisticsPage from './pages/StatisticsPage';
import AlertsPage from './pages/AlertsPage';
import SettingsPage from './pages/SettingsPage';
import { DashboardIcon, CubeIcon, ServerIcon, WalletIcon, ChartBarIcon, BellIcon, CogIcon } from './components/icons';

export type Page = 'dashboard' | 'halving' | 'rigs' | 'wallet' | 'statistics' | 'alerts' | 'settings';
export type Theme = 'light' | 'dark';

const App: React.FC = () => {
  const [activePage, setActivePage] = useState<Page>('dashboard');
  const [theme, setTheme] = useState<Theme>('dark');

  useEffect(() => {
    if (theme === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [theme]);

  const renderPage = () => {
    switch (activePage) {
      case 'dashboard':
        return <DashboardPage />;
      case 'halving':
        return <HalvingPage />;
      case 'rigs':
        return <RigsPage />;
      case 'wallet':
        return <WalletPage />;
      case 'statistics':
        return <StatisticsPage />;
      case 'alerts':
        return <AlertsPage />;
      case 'settings':
        return <SettingsPage currentTheme={theme} setTheme={setTheme} />;
      default:
        return <DashboardPage />;
    }
  };

  const NavItem = ({ page, label, icon }: { page: Page, label: string, icon: React.ReactElement }) => (
    <button
      onClick={() => setActivePage(page)}
      className={`flex flex-col md:flex-row items-center md:space-x-2 px-3 py-2 md:px-4 md:py-3 rounded-lg transition-colors duration-200 w-full text-left ${
        activePage === page
          ? 'bg-cyan-500/20 text-cyan-400'
          : 'text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700 hover:text-gray-900 dark:hover:text-white'
      }`}
    >
      {icon}
      <span className="font-medium text-xs md:text-sm mt-1 md:mt-0">{label}</span>
    </button>
  );

  return (
    <div className="min-h-screen bg-gray-100 dark:bg-gray-900 text-gray-800 dark:text-gray-200 font-sans">
      <div className="flex flex-col md:flex-row">
        <aside className="w-full md:w-64 bg-white dark:bg-gray-800 p-4 md:min-h-screen border-r border-gray-200 dark:border-gray-700">
          <div className="text-gray-900 dark:text-white text-2xl font-bold mb-6 md:mb-8 flex items-center justify-center md:justify-start">
            <span className="text-cyan-500 dark:text-cyan-400">Bit</span>Quan
          </div>
          <nav className="flex flex-row md:flex-col justify-around md:justify-start md:space-y-2 gap-2 md:gap-0">
            <NavItem page="dashboard" label="Dashboard" icon={<DashboardIcon />} />
            <NavItem page="halving" label="Block Progress" icon={<CubeIcon />} />
            <NavItem page="rigs" label="Your Rigs" icon={<ServerIcon />} />
            <NavItem page="wallet" label="Wallet" icon={<WalletIcon />} />
            <NavItem page="statistics" label="Statistics" icon={<ChartBarIcon />} />
            <NavItem page="alerts" label="Alerts" icon={<BellIcon />} />
            <NavItem page="settings" label="Settings" icon={<CogIcon />} />
          </nav>
        </aside>

        <main className="flex-1 p-4 sm:p-6 lg:p-8">
          {renderPage()}
        </main>
      </div>
    </div>
  );
};

export default App;