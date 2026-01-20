import React from 'react';
import { ElementType } from '../types';
import { ToolButton } from './ToolButton';
import '../styles/Toolbar.css';

interface ToolbarProps {
  currentTool: ElementType;
  selectTool: (tool: ElementType) => void;
  currentColor: string;
  strokeWidth: number;
  setWidth: (width: number) => void;
}

export const Toolbar: React.FC<ToolbarProps> = ({
  currentTool,
  selectTool,
  currentColor,
  strokeWidth,
  setWidth,
}) => {
  const tools: Array<{ type: ElementType; label: string }> = [
    { type: 'rectangle', label: 'Rectangle' },
    { type: 'circle', label: 'Circle' },
    { type: 'line', label: 'Line' },
    { type: 'arrow', label: 'Arrow' },
    { type: 'text', label: 'Text' },
  ];

  return (
    <div className="toolbar">
      <div className="toolbar-tools">
        {tools.map((tool) => (
          <ToolButton
            key={tool.type}
            type={tool.type}
            label={tool.label}
            isActive={currentTool === tool.type}
            onClick={() => selectTool(tool.type)}
          />
        ))}
      </div>

      <div className="toolbar-divider" />

      <div className="toolbar-options">
        <label>
          Stroke Width:
          <input
            type="range"
            min="1"
            max="10"
            value={strokeWidth}
            onChange={(e) => setWidth(Number(e.target.value))}
          />
          <span>{strokeWidth}px</span>
        </label>
      </div>
    </div>
  );
};
