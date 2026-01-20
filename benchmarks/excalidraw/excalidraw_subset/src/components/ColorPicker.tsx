import React from 'react';
import { PRESET_COLORS } from '../utils/colors';
import '../styles/ColorPicker.css';

interface ColorPickerProps {
  currentColor: string;
  onColorChange: (color: string) => void;
}

export const ColorPicker: React.FC<ColorPickerProps> = ({
  currentColor,
  onColorChange,
}) => {
  return (
    <div className="color-picker">
      <h3>Color</h3>
      <div className="color-grid">
        {PRESET_COLORS.map((color) => (
          <button
            key={color}
            className={`color-swatch ${currentColor === color ? 'active' : ''}`}
            style={{ backgroundColor: color }}
            onClick={() => onColorChange(color)}
            title={color}
          />
        ))}
      </div>
      <input
        type="color"
        value={currentColor}
        onChange={(e) => onColorChange(e.target.value)}
      />
    </div>
  );
};
