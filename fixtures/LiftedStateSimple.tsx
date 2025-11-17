// Simple Lifted State Example
// Demonstrates parent reading and writing child state

import { Component, state, setState } from '@minimact/core';

// Child component: Counter
// Uses regular useState - but parent can access its state!
export function Counter() {
  const [count, setCount] = useState(0);

  return (
    <div className="counter">
      <h3>Counter</h3>
      <p id="counter-display">Count: {count}</p>
      <button
        id="child-increment-btn"
        type="button"
        onClick={() => setCount(count + 1)}
      >
        Child Increment
      </button>
    </div>
  );
}

// Parent component: App
// Can read and modify child's state directly!
export default function App() {
  // Read child state using state["ComponentName.key"]
  const counterValue = state["Counter.count"];

  // Parent can modify child state using setState("ComponentName.key", value)
  const handleParentReset = () => {
    setState("Counter.count", 0);
  };

  const handleParentSetTo10 = () => {
    setState("Counter.count", 10);
  };

  return (
    <div id="app-root">
      <h1>Lifted State - Simple Example</h1>

      {/* Parent observes child state */}
      <div className="parent-display">
        <p>Parent sees counter value: <span id="parent-sees">{counterValue}</span></p>
      </div>

      {/* Parent controls for child state */}
      <div className="parent-controls">
        <button
          id="parent-reset-btn"
          type="button"
          onClick={handleParentReset}
        >
          Parent: Reset to 0
        </button>
        <button
          id="parent-set10-btn"
          type="button"
          onClick={handleParentSetTo10}
        >
          Parent: Set to 10
        </button>
      </div>

      {/* Child component wrapped with Component element */}
      <Component name="Counter" state={{ count: 0 }}>
        <Counter />
      </Component>

      {/* Status for testing */}
      <div id="status" className="status">
        <p>Counter Value: <span id="status-value">{counterValue}</span></p>
      </div>
    </div>
  );
}
