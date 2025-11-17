import { useState } from '@minimact/core';

/**
 * Test fixture for nested conditional rendering
 *
 * This tests how Minimact handles:
 * 1. Multiple levels of nested conditionals
 * 2. VNull nodes at different depths in the tree
 * 3. Hex path management when conditionals are nested
 * 4. Template patch system for complex conditional branches
 * 5. State synchronization across nested conditional boundaries
 */
export default function NestedConditionalsTest() {
  const [showOuter, setShowOuter] = useState(false);
  const [showMiddle, setShowMiddle] = useState(false);
  const [showInner, setShowInner] = useState(false);
  const [dynamicText, setDynamicText] = useState('Dynamic content');

  return (
    <div className="nested-conditionals-container">
      <h1>Nested Conditionals Test</h1>

      {/* Control panel */}
      <div className="controls">
        <button onClick={() => setShowOuter(!showOuter)}>
          Toggle Outer (currently: {showOuter ? 'true' : 'false'})
        </button>
        <button onClick={() => setShowMiddle(!showMiddle)}>
          Toggle Middle (currently: {showMiddle ? 'true' : 'false'})
        </button>
        <button onClick={() => setShowInner(!showInner)}>
          Toggle Inner (currently: {showInner ? 'true' : 'false'})
        </button>
        <input
          type="text"
          value={dynamicText}
          onChange={(e) => setDynamicText(e.target.value)}
          placeholder="Edit dynamic text"
        />
      </div>

      {/* State display */}
      <div className="state-display">
        <p>showOuter: {showOuter ? '✓ true' : '✗ false'}</p>
        <p>showMiddle: {showMiddle ? '✓ true' : '✗ false'}</p>
        <p>showInner: {showInner ? '✓ true' : '✗ false'}</p>
      </div>

      <hr />

      {/* Nested conditionals test - Level 1 */}
      {showOuter && (
        <div className="outer-level">
          <h2>Outer Level (showOuter = true)</h2>
          <p>This is the outer conditional block</p>
          <div className="outer-content">
            Outer content with dynamic value: <strong>{dynamicText}</strong>
          </div>

          {/* Nested conditionals test - Level 2 */}
          {showMiddle && (
            <div className="middle-level">
              <h3>Middle Level (showOuter && showMiddle = true)</h3>
              <p>This is the middle conditional block, nested inside outer</p>
              <div className="middle-content">
                <ul>
                  <li>Middle item 1: {dynamicText}</li>
                  <li>Middle item 2: {dynamicText.length} characters</li>
                </ul>
              </div>

              {/* Nested conditionals test - Level 3 */}
              {showInner && (
                <div className="inner-level">
                  <h4>Inner Level (showOuter && showMiddle && showInner = true)</h4>
                  <p>This is the innermost conditional block</p>
                  <div className="inner-content">
                    <div className="deeply-nested">
                      <span>Deeply nested dynamic content:</span>
                      <strong>{dynamicText}</strong>
                      <em> - Length: {dynamicText.length}</em>
                    </div>
                  </div>
                </div>
              )}

              <p className="after-inner">Content after inner conditional (still in middle)</p>
            </div>
          )}

          <p className="after-middle">Content after middle conditional (still in outer)</p>
        </div>
      )}

      {/* Test cases explanation */}
      <div className="test-cases">
        <h3>Test Scenarios:</h3>
        <ol>
          <li>
            <strong>All false (initial):</strong> All sections HIDDEN
            <br />
            <em>Tests VNull nodes at all three levels</em>
          </li>
          <li>
            <strong>Toggle outer to true:</strong> Outer SHOWN, middle/inner HIDDEN
            <br />
            <em>Tests replacing VNull with DOM tree containing nested VNull nodes</em>
          </li>
          <li>
            <strong>Toggle middle to true:</strong> Outer + Middle SHOWN, inner HIDDEN
            <br />
            <em>Tests replacing nested VNull with DOM tree containing deeper VNull</em>
          </li>
          <li>
            <strong>Toggle inner to true:</strong> All SHOWN
            <br />
            <em>Tests replacing deeply nested VNull with full DOM tree</em>
          </li>
          <li>
            <strong>Toggle inner back to false:</strong> Outer + Middle SHOWN, inner HIDDEN
            <br />
            <em>Tests replacing deeply nested DOM with VNull</em>
          </li>
          <li>
            <strong>Toggle middle back to false:</strong> Outer SHOWN, middle/inner HIDDEN
            <br />
            <em>Tests replacing nested DOM (with VNull inside) with VNull</em>
          </li>
          <li>
            <strong>Edit dynamicText at any visibility state:</strong>
            <br />
            <em>Tests state sync and dynamic updates at various nesting depths</em>
          </li>
          <li>
            <strong>Toggle states in different orders:</strong>
            <br />
            <em>Tests hex path stability when skipping levels (e.g., outer=true, inner=true, middle=false)</em>
          </li>
        </ol>
      </div>

      <hr />

      {/* Additional test: Sibling conditionals at same level */}
      <div className="sibling-conditionals">
        <h3>Sibling Conditionals Test</h3>
        {showOuter && (
          <div className="sibling-1">
            <p>First sibling (showOuter = true)</p>
          </div>
        )}
        {showMiddle && (
          <div className="sibling-2">
            <p>Second sibling (showMiddle = true)</p>
          </div>
        )}
        {showInner && (
          <div className="sibling-3">
            <p>Third sibling (showInner = true)</p>
          </div>
        )}
        <p>This tests hex path allocation for sibling VNull nodes</p>
      </div>
    </div>
  );
}
