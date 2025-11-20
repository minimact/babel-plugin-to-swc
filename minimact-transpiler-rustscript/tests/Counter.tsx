import React, { useState } from 'react';

interface CounterProps {
  initialValue?: number;
  label?: string;
}

function Counter({ initialValue = 0, label = "Count" }: CounterProps) {
  const [count, setCount] = useState(initialValue);
  const [isEnabled, setIsEnabled] = useState(true);

  const handleIncrement = () => {
    setCount(count + 1);
  };

  const handleDecrement = () => {
    setCount(count - 1);
  };

  return (
    <div className="counter">
      <h2>{label}</h2>
      <span>{count}</span>
      <button onClick={handleIncrement} disabled={!isEnabled}>+</button>
      <button onClick={handleDecrement} disabled={!isEnabled}>-</button>
    </div>
  );
}

export default Counter;
