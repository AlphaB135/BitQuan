import React from 'react';
import { useState } from 'react';

interface ToggleSwitchProps {
  initialChecked: boolean;
  onChange: (checked: boolean) => void;
}

const ToggleSwitch: React.FC<ToggleSwitchProps> = ({ initialChecked, onChange }) => {
  const [isChecked, setIsChecked] = useState(initialChecked);

  const handleToggle = () => {
    const newState = !isChecked;
    setIsChecked(newState);
    onChange(newState);
  };

  return (
    <label className="relative inline-flex items-center cursor-pointer group">
      <input 
        type="checkbox" 
        checked={isChecked} 
        onChange={handleToggle} 
        className="sr-only peer" 
        aria-label="Toggle switch"
      />
      <div className="w-11 h-6 bg-gray-200 dark:bg-gray-600 rounded-full peer peer-focus:ring-4 peer-focus:ring-cyan-500/50 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-cyan-500 group-hover:bg-gray-300 dark:group-hover:bg-gray-500 peer-checked:group-hover:bg-cyan-600"></div>
    </label>
  );
};

export default ToggleSwitch;