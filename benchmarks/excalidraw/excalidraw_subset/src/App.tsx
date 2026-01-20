import React from 'react';
import { Canvas } from './components/Canvas';
import { Toolbar } from './components/Toolbar';
import { Sidebar } from './components/Sidebar';
import { ColorPicker } from './components/ColorPicker';
import { LayerPanel } from './components/LayerPanel';
import { useCanvas } from './hooks/useCanvas';
import { useToolbar } from './hooks/useToolbar';
import { useSelection } from './hooks/useSelection';
import './styles/App.css';

export const App: React.FC = () => {
  const canvasState = useCanvas();
  const toolbarState = useToolbar();
  const selectionState = useSelection();

  return (
    <div className="app-container">
      <header className="app-header">
        <h1>Excalidraw Subset Benchmark</h1>
      </header>

      <div className="app-body">
        <Sidebar>
          <LayerPanel layers={canvasState.layers} />
          <ColorPicker
            currentColor={toolbarState.currentColor}
            onColorChange={toolbarState.setColor}
          />
        </Sidebar>

        <main className="app-main">
          <Toolbar {...toolbarState} />
          <Canvas
            {...canvasState}
            selectedTool={toolbarState.currentTool}
            selection={selectionState}
          />
        </main>
      </div>
    </div>
  );
};
