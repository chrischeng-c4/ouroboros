export interface RusheetError {
  code: string;
  message: string;
}

export interface CellData {
  value: string | null;
  displayValue: string;
  formula?: string;
  format: CellFormat;
  row: number;
  col: number;
}

export interface CellFormat {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  fontSize?: number;
  textColor?: string;
  backgroundColor?: string;
  horizontalAlign?: 'left' | 'center' | 'right';
  verticalAlign?: 'top' | 'middle' | 'bottom';
}

export interface Selection {
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
}

export interface Viewport {
  scrollX: number;
  scrollY: number;
  startRow: number;
  endRow: number;
  startCol: number;
  endCol: number;
  width: number;
  height: number;
}
