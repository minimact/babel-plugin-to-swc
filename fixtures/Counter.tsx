import { useState } from '@minimact/core';

export default function Counter() {
  const [count, setCount] = useState(0);
  const [message, setMessage] = useState("Hello");

  return (
    <div id="counter-root">
      <span id="counter-value">{count}</span>
      <span id="message">{message}</span>
      <button
        id="increment-btn"
        type="button"
        onClick={() => setCount(count + 1)}
      >
        Increment
      </button>
    </div>
  );
}
