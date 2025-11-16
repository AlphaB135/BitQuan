import React from 'react';

interface CardProps {
  children: React.ReactNode;
  className?: string;
}

const Card: React.FC<CardProps> = ({ children, className = '' }) => {
  return (
    <div className={`bg-white dark:bg-gray-800 rounded-xl shadow-md dark:shadow-lg p-4 md:p-6 transition-all duration-200 hover:shadow-lg dark:hover:shadow-xl ${className}`}>
      {children}
    </div>
  );
};

export default Card;