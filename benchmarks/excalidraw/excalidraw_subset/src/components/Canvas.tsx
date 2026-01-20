import React, { useRef, useEffect } from 'react';
import { Element, ElementType, Selection } from '../types';
import { renderElement } from './renderers';
import '../styles/Canvas.css';

interface CanvasProps {
  zoom: number;
  pan: { x: number; y: number };
  layers: any[];
  selectedTool: ElementType;
  selection: { selection: Selection };
}

export const Canvas: React.FC<CanvasProps> = ({
  zoom,
  pan,
  layers,
  selectedTool,
  selection,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Apply transformations
    ctx.save();
    ctx.translate(pan.x, pan.y);
    ctx.scale(zoom, zoom);

    // Render all layers
    for (const layer of layers) {
      if (!layer.visible) continue;

      for (const element of layer.elements) {
        renderElement(ctx, element);
      }
    }

    // Render selection bounds
    if (selection.selection.bounds) {
      const { min, max } = selection.selection.bounds;
      ctx.strokeStyle = '#4A90E2';
      ctx.lineWidth = 2 / zoom;
      ctx.setLineDash([5 / zoom, 5 / zoom]);
      ctx.strokeRect(
        min.x,
        min.y,
        max.x - min.x,
        max.y - min.y
      );
      ctx.setLineDash([]);
    }

    ctx.restore();
  }, [zoom, pan, layers, selection]);

  return (
    <div className="canvas-container">
      <canvas
        ref={canvasRef}
        width={1200}
        height={800}
        className="canvas"
      />
    </div>
  );
};
