import { useState } from '@minimact/core';

/**
 * Custom Hook: useTimer
 * Tests separate export (function defined first, exported later)
 */
function useTimer(namespace: string, initialSeconds: number = 0) {
  const [seconds, setSeconds] = useState(initialSeconds);

  const tick = () => setSeconds(seconds + 1);
  const reset = () => setSeconds(initialSeconds);

  const ui = (
    <div className="timer-widget">
      <span>Time: {seconds}s</span>
      <button onClick={tick}>Tick</button>
      <button onClick={reset}>Reset</button>
    </div>
  );

  return [seconds, tick, reset, ui];
}

export default useTimer;
