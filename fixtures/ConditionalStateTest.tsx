import { useState } from '@minimact/core';

/**
 * Test fixture for conditional rendering with multiple state dependencies
 *
 * This tests how Minimact handles:
 * 1. Conditional rendering with multiple boolean conditions (myState1 && !myState2)
 * 2. Initial state where condition is false (myState1 = false)
 * 3. Nested DOM elements within the conditional
 * 4. Dynamic content (myState3) inside the conditional block
 * 5. VNull node generation and hex path management
 * 6. Template patch system for conditional branches
 */
export default function ConditionalStateTest() {
  const [myState1, setMyState1] = useState(false);
  const [myState2, setMyState2] = useState(false);
  const [myState3, setMyState3] = useState('Initial text');

  return (
    <div className="conditional-test-container">
      <h1>Conditional State Test</h1>

      {/* Control panel */}
      <div className="controls">
        <button onClick={() => setMyState1(!myState1)}>
          Toggle myState1 (currently: {myState1 ? 'true' : 'false'})
        </button>
        <button onClick={() => setMyState2(!myState2)}>
          Toggle myState2 (currently: {myState2 ? 'true' : 'false'})
        </button>
        <input
          type="text"
          value={myState3}
          onChange={(e) => setMyState3(e.target.value)}
          placeholder="Edit myState3"
        />
      </div>

      {/* Current state display */}
      <div className="state-display">
        <p>myState1: {myState1 ? '✓ true' : '✗ false'}</p>
        <p>myState2: {myState2 ? '✓ true' : '✗ false'}</p>
        <p>myState3: "{myState3}"</p>
        <p>Condition (myState1 && !myState2): {myState1 && !myState2 ? '✓ SHOWN' : '✗ HIDDEN'}</p>
      </div>

      <hr />

      {/* The actual conditional rendering test case */}
      {myState1 && !myState2 && (
        <div className="conditional-content">
          <h2>Conditionally Rendered Section</h2>
          <p>This section is only visible when:</p>
          <ul>
            <li>myState1 is <strong>true</strong></li>
            <li>myState2 is <strong>false</strong></li>
          </ul>
          <div className="nested-content">
            <p>Some nested DOM elements here</p>
            <div className="dynamic-value">
              Dynamic content: <strong>{myState3}</strong>
            </div>
          </div>
        </div>
      )}

      {/* Test cases explanation */}
      <div className="test-cases">
        <h3>Test Scenarios:</h3>
        <ol>
          <li>
            <strong>Initial State:</strong> myState1=false, myState2=false → Section HIDDEN
            <br />
            <em>Tests VNull node generation when condition is initially false</em>
          </li>
          <li>
            <strong>Toggle myState1 to true:</strong> myState1=true, myState2=false → Section SHOWN
            <br />
            <em>Tests patch to replace VNull with full DOM tree</em>
          </li>
          <li>
            <strong>Toggle myState2 to true:</strong> myState1=true, myState2=true → Section HIDDEN
            <br />
            <em>Tests patch to replace DOM tree with VNull</em>
          </li>
          <li>
            <strong>Toggle myState2 back to false:</strong> myState1=true, myState2=false → Section SHOWN
            <br />
            <em>Tests patch to re-show the DOM tree</em>
          </li>
          <li>
            <strong>Edit myState3 while visible:</strong> Change text in input
            <br />
            <em>Tests state synchronization and dynamic content updates inside conditional</em>
          </li>
        </ol>
      </div>
    </div>
  );
}
