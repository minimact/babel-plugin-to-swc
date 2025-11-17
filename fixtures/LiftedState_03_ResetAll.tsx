// Pattern 2.1: Parent Modifying Child State - Reset All Button
// Parent can reset all child components at once

import { Component, state, setState } from '@minimact/core';

// Child component: Counter
export function Counter() {
  const count = state.count;

  return (
    <div className="counter">
      <h3>Counter</h3>
      <p>Count: <span id="counter-value">{count}</span></p>
      <button
        id="increment-btn"
        type="button"
        onClick={() => setState('count', count + 1)}
      >
        Increment
      </button>
    </div>
  );
}

// Child component: Timer
export function Timer() {
  const seconds = state.seconds;

  const handleTick = () => {
    setState('seconds', seconds + 1);
  };

  return (
    <div className="timer">
      <h3>Timer</h3>
      <p>Seconds: <span id="timer-value">{seconds}</span></p>
      <button
        id="tick-btn"
        type="button"
        onClick={handleTick}
      >
        Tick
      </button>
    </div>
  );
}

// Child component: Form
export function Form() {
  const text = state.text;

  return (
    <div className="form">
      <h3>Form</h3>
      <input
        id="text-input"
        type="text"
        placeholder="Enter text..."
        value={text}
        onInput={(e) => setState('text', e.target.value)}
      />
      <p>Length: <span id="text-length">{text.length}</span></p>
    </div>
  );
}

// Parent component: Dashboard
export default function Dashboard() {
  const counterValue = state["Counter.count"];
  const timerValue = state["Timer.seconds"];
  const formValue = state["Form.text"];

  const handleResetAll = () => {
    // Parent resets all child components at once
    setState("Counter.count", 0);
    setState("Timer.seconds", 0);
    setState("Form.text", "");
  };

  const handleResetCounter = () => {
    setState("Counter.count", 0);
  };

  const handleResetTimer = () => {
    setState("Timer.seconds", 0);
  };

  const handleResetForm = () => {
    setState("Form.text", "");
  };

  const hasChanges = counterValue !== 0 || timerValue !== 0 || formValue !== "";

  return (
    <div id="dashboard-root">
      <h1>Dashboard</h1>

      {/* Parent-level controls */}
      <div className="controls">
        <button
          id="reset-all-btn"
          type="button"
          onClick={handleResetAll}
          disabled={!hasChanges}
        >
          Reset All Components
        </button>

        <button
          id="reset-counter-btn"
          type="button"
          onClick={handleResetCounter}
          disabled={counterValue === 0}
        >
          Reset Counter Only
        </button>

        <button
          id="reset-timer-btn"
          type="button"
          onClick={handleResetTimer}
          disabled={timerValue === 0}
        >
          Reset Timer Only
        </button>

        <button
          id="reset-form-btn"
          type="button"
          onClick={handleResetForm}
          disabled={formValue === ""}
        >
          Reset Form Only
        </button>
      </div>

      <Component name="Counter" state={{ count: 0 }}>
        <Counter />
      </Component>

      <Component name="Timer" state={{ seconds: 0 }}>
        <Timer />
      </Component>

      <Component name="Form" state={{ text: "" }}>
        <Form />
      </Component>

      {/* Status display for testing */}
      <div id="status" className="status">
        <p>Counter: <span id="status-counter">{counterValue}</span></p>
        <p>Timer: <span id="status-timer">{timerValue}</span></p>
        <p>Form Text: <span id="status-form">"{formValue}"</span></p>
        <p>Has Changes: <span id="has-changes">{hasChanges ? 'Yes' : 'No'}</span></p>
      </div>
    </div>
  );
}
