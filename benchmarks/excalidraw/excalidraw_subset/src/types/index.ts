export interface Point {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface Element {
  id: string;
  type: ElementType;
  position: Point;
  size: Size;
  color: string;
  strokeWidth: number;
  rotation: number;
}

export type ElementType = 'rectangle' | 'circle' | 'line' | 'text' | 'arrow';

export interface Layer {
  id: string;
  name: string;
  elements: Element[];
  visible: boolean;
  locked: boolean;
}

export interface Tool {
  id: string;
  type: ElementType;
  icon: string;
  label: string;
}

export interface Selection {
  elements: Element[];
  bounds: {
    min: Point;
    max: Point;
  } | null;
}

export interface CanvasState {
  zoom: number;
  pan: Point;
  layers: Layer[];
  currentLayer: string | null;
}

export interface Color {
  hex: string;
  rgb: {
    r: number;
    g: number;
    b: number;
  };
  alpha: number;
}
