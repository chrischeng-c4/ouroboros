import { useState, useCallback } from 'react';
import { CanvasState, Layer, Point } from '../types';
import { generateId } from '../utils/uuid';

export function useCanvas() {
  const [state, setState] = useState<CanvasState>({
    zoom: 1,
    pan: { x: 0, y: 0 },
    layers: [
      {
        id: generateId(),
        name: 'Layer 1',
        elements: [],
        visible: true,
        locked: false,
      },
    ],
    currentLayer: null,
  });

  const setZoom = useCallback((zoom: number) => {
    setState((prev) => ({ ...prev, zoom }));
  }, []);

  const setPan = useCallback((pan: Point) => {
    setState((prev) => ({ ...prev, pan }));
  }, []);

  const addLayer = useCallback(() => {
    const newLayer: Layer = {
      id: generateId(),
      name: `Layer ${state.layers.length + 1}`,
      elements: [],
      visible: true,
      locked: false,
    };
    setState((prev) => ({
      ...prev,
      layers: [...prev.layers, newLayer],
    }));
  }, [state.layers.length]);

  const removeLayer = useCallback((layerId: string) => {
    setState((prev) => ({
      ...prev,
      layers: prev.layers.filter((l) => l.id !== layerId),
    }));
  }, []);

  return {
    ...state,
    setZoom,
    setPan,
    addLayer,
    removeLayer,
  };
}
