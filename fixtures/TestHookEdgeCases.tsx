import { useState } from '@minimact/core';
// Test Case 1: Default Export (already covered by useToggle)
import useToggle from './useToggle';

// Test Case 2: Named Export
import { useCounter } from './useCounter';

// Test Case 3: Renamed Import
import Timer from './useTimer';

// Test Case 4: Multiple Hooks from Same File
import { useDoubler } from './useCounter';

// Test Case 6: Inline + Imported Hooks
function useLocal(namespace: string, msg: string = 'Hello') {
  const [message, setMessage] = useState(msg);

  const update = () => setMessage(message + '!');

  const ui = (
    <div className="local-widget">
      <span>{message}</span>
      <button onClick={update}>Add !</button>
    </div>
  );

  return [message, update, ui];
}

/**
 * Component testing all edge cases for cross-file hook imports
 */
export default function TestHookEdgeCases() {
  // Test Case 1: Default Export
  const [isOn, , toggleUI] = useToggle('toggle1', false);

  // Test Case 2: Named Export
  const [count, , , , counterUI] = useCounter('counter1', 10);

  // Test Case 3: Renamed Import (useTimer imported as Timer)
  const [seconds, , , timerUI] = Timer('timer1', 5);

  // Test Case 4: Multiple hooks from same file
  const [doubled, , doublerUI] = useDoubler('doubler1', 3);

  // Test Case 6: Inline + Imported Hooks
  const [msg, , localUI] = useLocal('local1', 'Start');

  return (
    <div className="test-edge-cases">
      <h1>Hook Import Edge Cases Test</h1>

      <div className="section">
        <h2>1. Default Export (useToggle)</h2>
        <p>Toggle is: {isOn ? 'ON' : 'OFF'}</p>
        {toggleUI}
      </div>

      <div className="section">
        <h2>2. Named Export (useCounter)</h2>
        <p>Count: {count}</p>
        {counterUI}
      </div>

      <div className="section">
        <h2>3. Renamed Import (useTimer as Timer)</h2>
        <p>Seconds: {seconds}</p>
        {timerUI}
      </div>

      <div className="section">
        <h2>4. Multiple Hooks from Same File (useDoubler)</h2>
        <p>Doubled value: {doubled}</p>
        {doublerUI}
      </div>

      <div className="section">
        <h2>5. Inline + Imported Mix</h2>
        <p>Local message: {msg}</p>
        {localUI}
      </div>

      <div className="summary">
        <h3>Summary</h3>
        <p>All hook values: {isOn ? 1 : 0} + {count} + {seconds} + {doubled} = {(isOn ? 1 : 0) + count + seconds + doubled}</p>
      </div>
    </div>
  );
}
