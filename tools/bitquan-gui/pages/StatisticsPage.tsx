import React from 'react';
import Card from '../components/Card';

const dailyEarningsData = [
  { day: 'Mon', earnings: 170.5 },
  { day: 'Tue', earnings: 185.2 },
  { day: 'Wed', earnings: 160.8 },
  { day: 'Thu', earnings: 190.1 },
  { day: 'Fri', earnings: 210.6 },
  { day: 'Sat', earnings: 230.0 },
  { day: 'Sun', earnings: 225.4 },
];

const BarChart: React.FC<{ data: { day: string, earnings: number }[], title: string }> = ({ data, title }) => {
    const maxEarning = Math.max(...data.map(d => d.earnings));

    return (
        <Card>
            <h3 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">{title}</h3>
            <div className="flex justify-around items-end h-48 border-l border-b border-gray-200 dark:border-gray-700 p-2 overflow-x-auto">
                {data.map(item => (
                    <div key={item.day} className="flex flex-col items-center min-w-[40px] sm:min-w-[50px] md:min-w-[60px]">
                        <div className="text-xs text-gray-500 dark:text-gray-400 mb-1 font-mono hidden sm:block">${item.earnings.toFixed(0)}</div>
                        <div 
                            className="w-6 sm:w-8 bg-cyan-500 rounded-t-md hover:bg-cyan-400 transition-colors cursor-pointer" 
                            style={{ height: `${(item.earnings / maxEarning) * 100}%` }}
                            title={`${item.day}: ${item.earnings.toFixed(2)} BQ`}
                        >
                        </div>
                        <div className="mt-2 text-xs text-gray-500 dark:text-gray-400">{item.day}</div>
                    </div>
                ))}
            </div>
        </Card>
    );
}

const StatisticsPage: React.FC = () => {
  return (
    <div className="space-y-8">
      <h1 className="text-3xl font-bold text-gray-900 dark:text-white">📊 Statistics</h1>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <Card>
              <h2 className="text-lg font-semibold text-cyan-500 dark:text-cyan-400 mb-2">Total Hashrate</h2>
              <p className="text-2xl font-bold">2.84 MH/s</p>
              <p className="text-md text-gray-500 dark:text-gray-400">Across 3 active rigs</p>
          </Card>
          <Card>
              <h2 className="text-lg font-semibold text-cyan-500 dark:text-cyan-400 mb-2">24h Earnings</h2>
              <p className="text-2xl font-bold">295.60 BQ</p>
              <p className="text-md text-gray-500 dark:text-gray-400 text-green-500">+5.2% vs yesterday</p>
          </Card>
          <Card>
              <h2 className="text-lg font-semibold text-cyan-500 dark:text-cyan-400 mb-2">Mining Efficiency</h2>
              <p className="text-2xl font-bold">98.7%</p>
              <p className="text-md text-gray-500 dark:text-gray-400">Accepted Shares</p>
          </Card>
      </div>

      <div className="grid grid-cols-1 gap-6">
        <BarChart data={dailyEarningsData} title="Last 7 Days Earnings (BQ)" />
      </div>

        <Card>
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">Rig Performance</h2>
            <div className="overflow-x-auto">
                <table className="w-full text-left min-w-[400px]">
                    <thead className="border-b border-gray-200 dark:border-gray-600 text-gray-500 dark:text-gray-400">
                    <tr>
                        <th className="p-2 md:p-3">Rig Name</th>
                        <th className="p-2 md:p-3">Hashrate</th>
                        <th className="p-2 md:p-3">Efficiency</th>
                        <th className="p-2 md:p-3">Earnings</th>
                    </tr>
                    </thead>
                    <tbody>
                        <tr className="border-b border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700/50">
                            <td className="p-2 md:p-3 font-medium text-sm">Node-01</td>
                            <td className="p-2 md:p-3 font-mono text-sm">1.2 MH/s</td>
                            <td className="p-2 md:p-3 text-green-500 dark:text-green-400 text-sm">99.1%</td>
                            <td className="p-2 md:p-3 font-semibold text-sm">125.5 BQ</td>
                        </tr>
                         <tr className="border-b border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700/50">
                            <td className="p-2 md:p-3 font-medium text-sm">Node-02</td>
                            <td className="p-2 md:p-3 font-mono text-sm">1.19 MH/s</td>
                            <td className="p-2 md:p-3 text-green-500 dark:text-green-400 text-sm">98.9%</td>
                            <td className="p-2 md:p-3 font-semibold text-sm">124.9 BQ</td>
                        </tr>
                         <tr className="border-b border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700/50">
                            <td className="p-2 md:p-3 font-medium text-sm">Node-CPU</td>
                            <td className="p-2 md:p-3 font-mono text-sm">455 kH/s</td>
                            <td className="p-2 md:p-3 text-yellow-500 dark:text-yellow-400 text-sm">97.5%</td>
                            <td className="p-2 md:p-3 font-semibold text-sm">45.2 BQ</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </Card>
    </div>
  );
};

export default StatisticsPage;
