import { Element } from '../types';

export function renderElement(ctx: CanvasRenderingContext2D, element: Element): void {
  ctx.save();

  ctx.fillStyle = element.color;
  ctx.strokeStyle = element.color;
  ctx.lineWidth = element.strokeWidth;

  const { x, y } = element.position;
  const { width, height } = element.size;

  switch (element.type) {
    case 'rectangle':
      ctx.strokeRect(x, y, width, height);
      break;

    case 'circle':
      const radiusX = width / 2;
      const radiusY = height / 2;
      const centerX = x + radiusX;
      const centerY = y + radiusY;
      ctx.beginPath();
      ctx.ellipse(centerX, centerY, radiusX, radiusY, 0, 0, 2 * Math.PI);
      ctx.stroke();
      break;

    case 'line':
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + width, y + height);
      ctx.stroke();
      break;

    case 'arrow':
      // Draw line
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + width, y + height);
      ctx.stroke();

      // Draw arrowhead
      const angle = Math.atan2(height, width);
      const arrowLength = 15;
      const arrowAngle = Math.PI / 6;

      ctx.beginPath();
      ctx.moveTo(x + width, y + height);
      ctx.lineTo(
        x + width - arrowLength * Math.cos(angle - arrowAngle),
        y + height - arrowLength * Math.sin(angle - arrowAngle)
      );
      ctx.moveTo(x + width, y + height);
      ctx.lineTo(
        x + width - arrowLength * Math.cos(angle + arrowAngle),
        y + height - arrowLength * Math.sin(angle + arrowAngle)
      );
      ctx.stroke();
      break;

    case 'text':
      ctx.font = '16px sans-serif';
      ctx.fillText('Text', x, y);
      break;
  }

  ctx.restore();
}
