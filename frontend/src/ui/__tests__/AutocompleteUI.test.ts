import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { AutocompleteUI } from '../AutocompleteUI';
import type { Suggestion } from '../AutocompleteEngine';

describe('AutocompleteUI', () => {
  let ui: AutocompleteUI;
  let container: HTMLElement;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    ui = new AutocompleteUI(container);

    // Mock scrollIntoView for JSDOM
    Element.prototype.scrollIntoView = function() {};
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  it('renders suggestions', () => {
    const suggestions: Suggestion[] = [
      { type: 'function', value: 'SUM', display: 'SUM()', insertText: 'SUM(' },
      { type: 'function', value: 'AVERAGE', display: 'AVERAGE()', insertText: 'AVERAGE(' },
    ];

    ui.show(suggestions, { x: 100, y: 200 });

    const dropdown = container.querySelector('.autocomplete-dropdown');
    expect(dropdown).toBeTruthy();
    expect(dropdown?.children.length).toBe(2);
  });

  it('highlights selected suggestion', () => {
    const suggestions: Suggestion[] = [
      { type: 'function', value: 'SUM', display: 'SUM()', insertText: 'SUM(' },
      { type: 'function', value: 'AVERAGE', display: 'AVERAGE()', insertText: 'AVERAGE(' },
    ];

    ui.show(suggestions, { x: 100, y: 200 });
    ui.navigate(1);

    const items = container.querySelectorAll('.autocomplete-item');
    expect(items[1].classList.contains('selected')).toBe(true);
    expect(items[0].classList.contains('selected')).toBe(false);
  });

  it('returns selected suggestion', () => {
    const suggestions: Suggestion[] = [
      { type: 'function', value: 'SUM', display: 'SUM()', insertText: 'SUM(' },
    ];

    ui.show(suggestions, { x: 100, y: 200 });
    const selected = ui.getSelected();

    expect(selected?.value).toBe('SUM');
  });

  it('hides dropdown', () => {
    const suggestions: Suggestion[] = [
      { type: 'function', value: 'SUM', display: 'SUM()', insertText: 'SUM(' },
    ];

    ui.show(suggestions, { x: 100, y: 200 });
    expect(ui.isVisible()).toBe(true);

    ui.hide();
    expect(ui.isVisible()).toBe(false);
  });

  it('navigates with wrapping', () => {
    const suggestions: Suggestion[] = [
      { type: 'function', value: 'SUM', display: 'SUM()', insertText: 'SUM(' },
      { type: 'function', value: 'AVERAGE', display: 'AVERAGE()', insertText: 'AVERAGE(' },
    ];

    ui.show(suggestions, { x: 100, y: 200 });

    ui.navigate(1);
    expect(ui.getSelectedIndex()).toBe(1);

    ui.navigate(1);
    expect(ui.getSelectedIndex()).toBe(0);

    ui.navigate(-1);
    expect(ui.getSelectedIndex()).toBe(1);
  });
});
