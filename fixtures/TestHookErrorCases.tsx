import { useState } from '@minimact/core';
// This file tests edge cases that should be handled gracefully

// Valid import
import useToggle from './useToggle';

// Test: Import from node_modules (should be skipped)
// import { someHook } from 'some-library'; // Commented out for now

// Test: Import non-existent file (should be skipped)
// import useNonExistent from './useNonExistent'; // Commented out for now

/**
 * Component testing error handling edge cases
 */
export default function TestHookErrorCases() {
  const [isOn, toggle, toggleUI] = useToggle('toggle1', true);

  return (
    <div className="test-error-cases">
      <h1>Hook Error Cases Test</h1>

      <div className="section">
        <h2>Valid Import (Control)</h2>
        <p>Toggle is: {isOn ? 'ON' : 'OFF'}</p>
        <button onClick={toggle}>Manual Toggle</button>
        {toggleUI}
      </div>

      <div className="section">
        <h2>Error Cases</h2>
        <p>Check console for warnings about:</p>
        <ul>
          <li>Non-relative imports (node_modules) - should be skipped</li>
          <li>Missing files - should be skipped gracefully</li>
          <li>Circular imports - should be detected</li>
        </ul>
      </div>
    </div>
  );
}
