import { useState, useCallback } from 'react';
import { ElementType } from '../types';

export function useToolbar() {
  const [currentTool, setCurrentTool] = useState<ElementType>('rectangle');
  const [currentColor, setCurrentColor] = useState<string>('#000000');
  const [strokeWidth, setStrokeWidth] = useState<number>(2);

  const selectTool = useCallback((tool: ElementType) => {
    setCurrentTool(tool);
  }, []);

  const setColor = useCallback((color: string) => {
    setCurrentColor(color);
  }, []);

  const setWidth = useCallback((width: number) => {
    setStrokeWidth(width);
  }, []);

  return {
    currentTool,
    currentColor,
    strokeWidth,
    selectTool,
    setColor,
    setWidth,
  };
}
