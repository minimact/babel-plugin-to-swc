import { useState } from '@minimact/core';

/**
 * Custom Hook: useCounter with UI
 * Tests named exports and multiple returns
 *
 * @param namespace - Unique identifier for this hook instance (REQUIRED)
 * @param start - Initial count value
 * @returns [count, increment, decrement, reset, ui]
 */
export function useCounter(namespace: string, start: number = 0) {
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
 * Another hook in the same file - useDoubler
 */
export function useDoubler(namespace: string, initial: number = 0) {
  const [value, setValue] = useState(initial);

  const double = () => setValue(value * 2);

  const ui = (
    <div className="doubler-widget">
      <span>Value: {value}</span>
      <button onClick={double}>Double</button>
    </div>
  );

  return [value, double, ui];
}
