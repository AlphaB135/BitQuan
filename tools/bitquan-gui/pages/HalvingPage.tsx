import React from 'react';
import { useState, useEffect } from 'react';
import Card from '../components/Card';

// Total blocks to display in the grid
const TOTAL_BLOCKS_IN_GRID = 350; 
// Starting block height for the simulation
const INITIAL_BLOCK_HEIGHT = 312; 

type BlockStatus = 'mined' | 'mining' | 'unmined';

// A single block in the grid, now with a block number
const Block: React.FC<{ status: BlockStatus; blockNumber: number }> = React.memo(({ status, blockNumber }) => {
  const baseClasses = 'w-full h-full rounded transition-colors duration-300 flex items-center justify-center p-0.5 md:p-1 select-none';
  let statusClasses = '';
  let textClasses = 'font-mono text-center';

  switch (status) {
    case 'mined':
      statusClasses = 'bg-gray-200 dark:bg-gray-800 border border-cyan-300 dark:border-cyan-800';
      textClasses += ' text-cyan-800 dark:text-cyan-600 text-xs md:text-xs';
      break;
    case 'mining':
      statusClasses = 'bg-cyan-500 animate-pulse border border-cyan-300 shadow-lg shadow-cyan-500/50';
      textClasses += ' text-white font-bold text-xs md:text-sm';
      break;
    case 'unmined':
      statusClasses = 'bg-gray-100 dark:bg-gray-700/50 border border-gray-200 dark:border-gray-600/80';
      textClasses += ' text-gray-400 dark:text-gray-500 text-xs';
      break;
  }
  
  return (
    <div className={`${baseClasses} ${statusClasses}`}>
      <span className={`${textClasses} hidden sm:inline`}>{blockNumber}</span>
      <span className={`${textClasses} sm:hidden text-xs`}>{blockNumber % 100}</span>
    </div>
  );
});


const HalvingPage: React.FC = () => {
  const [currentBlockHeight, setCurrentBlockHeight] = useState(INITIAL_BLOCK_HEIGHT);

  useEffect(() => {
    // Simulate finding a new block every 1.5 seconds
    const interval = setInterval(() => {
      setCurrentBlockHeight(prev => (prev < TOTAL_BLOCKS_IN_GRID -1 ? prev + 1 : prev));
    }, 1500); 

    return () => clearInterval(interval);
  }, []);

  const progressPercentage = ((currentBlockHeight + 1) / TOTAL_BLOCKS_IN_GRID) * 100;

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Block Progress</h1>
      
      {/* Top section with stats and legend */}
      <Card>
        <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-6">
            <div className="flex-1">
                <h2 className="text-xl font-semibold text-cyan-500 dark:text-cyan-400 mb-2">Current Block Height</h2>
                <p className="text-4xl font-mono font-bold">{currentBlockHeight.toLocaleString()}</p>
                 <div className="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2.5 mt-3">
                    <div className="bg-cyan-500 h-2.5 rounded-full" style={{ width: `${progressPercentage}%` }}></div>
                </div>
            </div>

            <div className="flex items-center space-x-6 text-sm text-gray-500 dark:text-gray-400">
                <div className="flex items-center space-x-2">
                    <div className="w-4 h-4 rounded-sm bg-gray-200 dark:bg-cyan-800 border border-cyan-300 dark:border-cyan-700"></div>
                    <span>Mined</span>
                </div>
                <div className="flex items-center space-x-2">
                    <div className="w-4 h-4 rounded-sm bg-cyan-500"></div>
                    <span>Mining</span>
                </div>
                <div className="flex items-center space-x-2">
                    <div className="w-4 h-4 rounded-sm bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600"></div>
                    <span>Unmined</span>
                </div>
            </div>
        </div>
      </Card>
      
      {/* Grid of blocks */}
      <Card className="p-2 md:p-4">
        <div className="grid gap-1 md:gap-2 grid-cols-8 sm:grid-cols-10 md:grid-cols-12 lg:grid-cols-14 xl:grid-cols-16">
          {Array.from({ length: TOTAL_BLOCKS_IN_GRID }).map((_, index) => {
            let status: BlockStatus = 'unmined';
            if (index < currentBlockHeight) {
                status = 'mined';
            } else if (index === currentBlockHeight) {
                status = 'mining';
            }
            return (
                <div key={index} className="aspect-square">
                  <Block status={status} blockNumber={index} />
                </div>
            );
        })}
        </div>
      </Card>
    </div>
  );
};

export default HalvingPage;