import React from 'react';
import { ElementType } from '../types';
import '../styles/ToolButton.css';

interface ToolButtonProps {
  type: ElementType;
  label: string;
  isActive: boolean;
  onClick: () => void;
}

export const ToolButton: React.FC<ToolButtonProps> = ({
  type,
  label,
  isActive,
  onClick,
}) => {
  return (
    <button
      className={`tool-button ${isActive ? 'active' : ''}`}
      onClick={onClick}
      title={label}
    >
      {label}
    </button>
  );
};
