/**
 * SpatialLayout - Test fixture for useArea (Citrine Ranger)
 *
 * Tests minimact-spatial: Viewport as 2D database with spatial queries
 *
 * Philosophy: Query SPACE, not elements. Track REGIONS reactively.
 */

import React from 'react';
import { useArea } from 'minimact-spatial';

export function SpatialLayout() {
  // Define spatial areas
  const header = useArea({ top: 0, height: 80 });
  const sidebar = useArea('#sidebar-area');
  const mainContent = useArea('#main-content');
  const viewport = useArea('viewport');

  return (
    <div className="spatial-layout" data-testid="spatial-layout">
      {/* Header area */}
      <header
        id="header-area"
        style={{ height: '80px', background: '#f0f0f0' }}
        data-testid="header"
      >
        <h1>Spatial Computing Demo</h1>
        <nav>
          <a href="#">Link 1</a>
          <a href="#">Link 2</a>
          <a href="#">Link 3</a>
        </nav>
      </header>

      <div style={{ display: 'flex' }}>
        {/* Sidebar area */}
        <aside
          id="sidebar-area"
          style={{ width: '250px', minHeight: '500px', background: '#e0e0e0' }}
          data-testid="sidebar"
        >
          <h2>Sidebar</h2>
          <ul>
            <li>Item 1</li>
            <li>Item 2</li>
            <li>Item 3</li>
            <li>Item 4</li>
            <li>Item 5</li>
            <li>Item 6</li>
            <li>Item 7</li>
            <li>Item 8</li>
            <li>Item 9</li>
            <li>Item 10</li>
          </ul>
        </aside>

        {/* Main content area */}
        <main
          id="main-content"
          style={{ flex: 1, padding: '20px' }}
          data-testid="main-content"
        >
          <h2>Main Content</h2>

          {/* Spatial Stats Display */}
          <div className="spatial-stats" data-testid="spatial-stats">
            <h3>Header Stats</h3>
            <p data-testid="header-elements-count">
              Elements: {header.elementsCount}
            </p>
            <p data-testid="header-coverage">
              Coverage: {(header.coverage * 100).toFixed(1)}%
            </p>
            <p data-testid="header-is-full">
              Is Full: {header.isFull ? 'Yes' : 'No'}
            </p>
            <p data-testid="header-is-empty">
              Is Empty: {header.isEmpty ? 'Yes' : 'No'}
            </p>

            <h3>Sidebar Stats</h3>
            <p data-testid="sidebar-elements-count">
              Elements: {sidebar.elementsCount}
            </p>
            <p data-testid="sidebar-coverage">
              Coverage: {(sidebar.coverage * 100).toFixed(1)}%
            </p>
            <p data-testid="sidebar-density">
              Density: {sidebar.elementDensity.toFixed(2)} elements/1000px²
            </p>

            <h3>Main Content Stats</h3>
            <p data-testid="main-elements-count">
              Elements: {mainContent.elementsCount}
            </p>
            <p data-testid="main-average-size">
              Avg Element Size: {mainContent.averageElementSize.toFixed(0)}px²
            </p>

            <h3>Viewport Stats</h3>
            <p data-testid="viewport-width">
              Width: {viewport.width}px
            </p>
            <p data-testid="viewport-height">
              Height: {viewport.height}px
            </p>
            <p data-testid="viewport-total-elements">
              Total Elements: {viewport.elementsCount}
            </p>
          </div>

          {/* Conditional rendering based on spatial queries */}
          <div className="spatial-indicators" data-testid="spatial-indicators">
            {header.isFull && (
              <div data-testid="header-full-indicator" className="indicator">
                ⚠️ Header is full - consider compact mode
              </div>
            )}

            {sidebar.elementsCount > 10 && (
              <div data-testid="sidebar-scroll-indicator" className="indicator">
                📜 Sidebar has many items - show scroll indicator
              </div>
            )}

            {mainContent.isEmpty && (
              <div data-testid="main-empty-indicator" className="indicator">
                📭 Main content is empty - show empty state
              </div>
            )}

            {viewport.isSparse && (
              <div data-testid="viewport-sparse-indicator" className="indicator">
                🌌 Viewport is sparse - lots of empty space
              </div>
            )}
          </div>

          {/* Collision detection demo */}
          <div className="collision-demo" data-testid="collision-demo">
            <h3>Collision Detection</h3>
            <p data-testid="sidebar-main-overlap">
              Sidebar overlaps main: {sidebar.overlaps(mainContent) ? 'Yes' : 'No'}
            </p>
            <p data-testid="header-sidebar-overlap">
              Header overlaps sidebar: {header.overlaps(sidebar) ? 'Yes' : 'No'}
            </p>
          </div>

          {/* Test content to populate areas */}
          <div className="content-cards">
            <div className="card">Card 1</div>
            <div className="card">Card 2</div>
            <div className="card">Card 3</div>
          </div>
        </main>
      </div>

      {/* Controls for testing */}
      <div className="controls" data-testid="controls">
        <button data-action="add-header-items">
          Add Header Items
        </button>
        <button data-action="add-sidebar-items">
          Add Sidebar Items
        </button>
        <button data-action="clear-main-content">
          Clear Main Content
        </button>
        <button data-action="resize-viewport">
          Resize Viewport
        </button>
      </div>
    </div>
  );
}
