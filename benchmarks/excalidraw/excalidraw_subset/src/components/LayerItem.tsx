import React from 'react';
import { Layer } from '../types';
import '../styles/LayerItem.css';

interface LayerItemProps {
  layer: Layer;
}

export const LayerItem: React.FC<LayerItemProps> = ({ layer }) => {
  return (
    <div className={`layer-item ${!layer.visible ? 'hidden' : ''}`}>
      <span className="layer-name">{layer.name}</span>
      <div className="layer-controls">
        <button title="Toggle visibility">👁</button>
        <button title="Lock layer">🔒</button>
      </div>
    </div>
  );
};
