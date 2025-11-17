import { useState } from '@minimact/core';

/**
 * Custom Hook: useCounter with UI
 * Tests the custom hooks-as-components system
 */
function useCounter(namespace: string, start: number = 0) {
  const [count, setCount] = useState(start);

  const increment = () => setCount(count + 1);
  const decrement = () => setCount(count - 1);
  const reset = () => setCount(start);

  const ui = (
    <div className="counter-widget">
      <button onClick={decrement}>-</button>
      <span className="count-display">{count}</span>
      <button onClick={increment}>+</button>
      <button onClick={reset}>Reset</button>
    </div>
  );

  return [count, increment, decrement, reset, ui];
}

/**
 * Component using the custom hook
 */
export default function TestCustomHook() {
  const [count1, increment1, , , counterUI1] = useCounter('counter1', 0);
  const [count2, increment2, , , counterUI2] = useCounter('counter2', 10);

  return (
    <div className="app">
      <h1>Custom Hooks Test</h1>

      <div className="section">
        <h2>Counter 1 (starts at 0)</h2>
        <p>External count: {count1}</p>
        <button onClick={increment1}>External +1</button>
        {counterUI1}
      </div>

      <div className="section">
        <h2>Counter 2 (starts at 10)</h2>
        <p>External count: {count2}</p>
        <button onClick={increment2}>External +1</button>
        {counterUI2}
      </div>
    </div>
  );
}
