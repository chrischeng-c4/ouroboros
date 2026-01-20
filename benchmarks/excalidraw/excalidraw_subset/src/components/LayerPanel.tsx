import React from 'react';
import { Layer } from '../types';
import { LayerItem } from './LayerItem';
import '../styles/LayerPanel.css';

interface LayerPanelProps {
  layers: Layer[];
}

export const LayerPanel: React.FC<LayerPanelProps> = ({ layers }) => {
  return (
    <div className="layer-panel">
      <h3>Layers</h3>
      <div className="layer-list">
        {layers.map((layer) => (
          <LayerItem key={layer.id} layer={layer} />
        ))}
      </div>
    </div>
  );
};
