import type { CollabUser, CollaborationProvider } from './CollaborationProvider';

/**
 * UI component for displaying collaboration status and connected users
 */
export class CollaborationUI {
  private container: HTMLElement;
  private provider: CollaborationProvider;
  private unsubscribe: (() => void) | null = null;

  constructor(container: HTMLElement, provider: CollaborationProvider) {
    this.container = container;
    this.provider = provider;
    this.render();
    this.subscribeToChanges();
  }

  /**
   * Subscribe to user changes
   */
  private subscribeToChanges(): void {
    this.unsubscribe = this.provider.onUsersChange((users) => {
      this.renderUsers(users);
    });
  }

  /**
   * Render the collaboration UI
   */
  private render(): void {
    this.container.innerHTML = `
      <div class="collab-container">
        <div class="collab-status">
          <span class="collab-dot"></span>
          <span class="collab-text">Connecting...</span>
        </div>
        <div class="collab-users"></div>
      </div>
    `;

    this.addStyles();
    this.updateStatus();
  }

  /**
   * Add CSS styles
   */
  private addStyles(): void {
    if (document.getElementById('collab-styles')) return;

    const style = document.createElement('style');
    style.id = 'collab-styles';
    style.textContent = `
      .collab-container {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 8px 12px;
        background: #f8f9fa;
        border-bottom: 1px solid #e0e0e0;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        font-size: 13px;
      }

      .collab-status {
        display: flex;
        align-items: center;
        gap: 6px;
      }

      .collab-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: #ffc107;
        transition: background 0.3s;
      }

      .collab-dot.connected {
        background: #28a745;
      }

      .collab-dot.disconnected {
        background: #dc3545;
      }

      .collab-text {
        color: #666;
      }

      .collab-users {
        display: flex;
        gap: 4px;
      }

      .collab-user {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        border-radius: 50%;
        color: white;
        font-weight: 600;
        font-size: 11px;
        cursor: default;
        transition: transform 0.2s;
      }

      .collab-user:hover {
        transform: scale(1.1);
      }

      .collab-user[title] {
        position: relative;
      }
    `;
    document.head.appendChild(style);
  }

  /**
   * Update connection status
   */
  updateStatus(): void {
    const dot = this.container.querySelector('.collab-dot');
    const text = this.container.querySelector('.collab-text');

    if (!dot || !text) return;

    if (this.provider.isConnected()) {
      dot.className = 'collab-dot connected';
      text.textContent = 'Connected';
    } else {
      dot.className = 'collab-dot disconnected';
      text.textContent = 'Disconnected';
    }
  }

  /**
   * Render connected users
   */
  private renderUsers(users: CollabUser[]): void {
    const usersContainer = this.container.querySelector('.collab-users');
    if (!usersContainer) return;

    usersContainer.innerHTML = users.map(user => `
      <div
        class="collab-user"
        style="background-color: ${user.color}"
        title="${user.name}"
      >
        ${user.name.charAt(0).toUpperCase()}
      </div>
    `).join('');

    // Update status text to show user count
    const text = this.container.querySelector('.collab-text');
    if (text && this.provider.isConnected()) {
      const count = users.length;
      text.textContent = count > 0
        ? `Connected (${count + 1} user${count > 0 ? 's' : ''})`
        : 'Connected';
    }
  }

  /**
   * Cleanup
   */
  destroy(): void {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
    this.container.innerHTML = '';
  }
}

/**
 * Create and mount collaboration UI
 */
export function createCollaborationUI(
  provider: CollaborationProvider,
  mountPoint?: HTMLElement
): CollaborationUI {
  const container = mountPoint || document.createElement('div');

  if (!mountPoint) {
    container.id = 'collab-ui';
    // Insert at the top of the body
    document.body.insertBefore(container, document.body.firstChild);
  }

  return new CollaborationUI(container, provider);
}
