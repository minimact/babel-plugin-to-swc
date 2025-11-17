import { useRef, useEffect, useState } from '@minimact/core';

export function QuantumSlider() {
  const sliderRef = useRef<HTMLInputElement>(null);
  const [currentValue, setCurrentValue] = useState(50);
  const [isEntangled, setIsEntangled] = useState(false);
  const [entangledWith, setEntangledWith] = useState('');

  useEffect(() => {
    console.log('[QuantumSlider] Component mounted');

    // Listen for value changes from slider
    const slider = sliderRef.current;
    if (slider) {
      const handleInput = (e: Event) => {
        const value = parseInt((e.target as HTMLInputElement).value);
        setCurrentValue(value);
        console.log(`[QuantumSlider] Value changed to: ${value}`);
      };

      slider.addEventListener('input', handleInput);

      return () => {
        slider.removeEventListener('input', handleInput);
      };
    }
  }, []);

  return (
    <div className="quantum-slider">
      <h1>⚪ Quantum Entangled Slider</h1>
      <p className="subtitle">
        This slider exists in <strong>multiple spacetime coordinates simultaneously</strong>
      </p>

      {/* Entanglement Status */}
      <div className="entanglement-status">
        <div className={`status-indicator ${isEntangled ? 'entangled' : 'isolated'}`}>
          <span className="status-dot"></span>
          <span className="status-text">
            {isEntangled ? `🔗 Entangled with: ${entangledWith}` : '❌ Not Entangled'}
          </span>
        </div>
      </div>

      {/* The Quantum Slider */}
      <div className="slider-container">
        <label htmlFor="slider-root">Volume Control:</label>
        <input
          ref={sliderRef}
          type="range"
          id="slider-root"
          className="quantum-range"
          min="0"
          max="100"
          defaultValue="50"
        />
      </div>

      {/* Value Display */}
      <div className="value-display">
        <div className="value-box">
          <span className="value-label">Current Value:</span>
          <span id="slider-value" className="value-number">{currentValue}</span>
        </div>
      </div>

      {/* Quantum Properties */}
      <div className="quantum-info">
        <h3>🌌 Quantum Properties</h3>
        <ul>
          <li>
            <strong>Identity Sync:</strong> This element shares the SAME identity across clients
          </li>
          <li>
            <strong>Mutation Vectors:</strong> Changes transmitted as differential operations
          </li>
          <li>
            <strong>Bidirectional:</strong> Alice moves slider → Bob's moves instantly (and vice versa)
          </li>
          <li>
            <strong>Bandwidth:</strong> 100x reduction vs full state sync (only deltas)
          </li>
        </ul>
      </div>

      {/* Instructions */}
      <div className="instructions">
        <h3>📋 How It Works</h3>
        <ol>
          <li>Two clients (Alice & Bob) connect to the hub</li>
          <li>PearlRanger calls <code>quantum.entangle()</code> to link the sliders</li>
          <li>Alice moves her slider → Mutation vector generated</li>
          <li>RealHub receives vector and forwards to Bob's V8 engine</li>
          <li>Bob's slider updates instantly (no full re-render)</li>
          <li>Same process happens in reverse (bidirectional)</li>
        </ol>
      </div>

      {/* Debug Info */}
      <details className="debug-info">
        <summary>Debug Info</summary>
        <pre>
          {JSON.stringify({
            sliderValue: currentValue,
            isEntangled,
            entangledWith,
            hasRef: !!sliderRef.current
          }, null, 2)}
        </pre>
      </details>

      <style>{`
        .quantum-slider {
          font-family: 'Segoe UI', system-ui, sans-serif;
          max-width: 600px;
          margin: 20px auto;
          padding: 20px;
          background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
          border-radius: 12px;
          box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
          color: white;
        }

        .subtitle {
          color: rgba(255, 255, 255, 0.9);
          font-size: 14px;
          margin-top: -10px;
        }

        .entanglement-status {
          background: rgba(0, 0, 0, 0.2);
          padding: 12px;
          border-radius: 8px;
          margin: 20px 0;
        }

        .status-indicator {
          display: flex;
          align-items: center;
          gap: 10px;
        }

        .status-dot {
          width: 12px;
          height: 12px;
          border-radius: 50%;
          background: #ef4444;
          animation: pulse 2s ease-in-out infinite;
        }

        .status-indicator.entangled .status-dot {
          background: #10b981;
        }

        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }

        .slider-container {
          margin: 30px 0;
        }

        .slider-container label {
          display: block;
          margin-bottom: 10px;
          font-weight: 500;
        }

        .quantum-range {
          width: 100%;
          height: 8px;
          border-radius: 4px;
          background: rgba(255, 255, 255, 0.2);
          outline: none;
          -webkit-appearance: none;
        }

        .quantum-range::-webkit-slider-thumb {
          -webkit-appearance: none;
          width: 24px;
          height: 24px;
          border-radius: 50%;
          background: white;
          cursor: pointer;
          box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
          transition: transform 0.1s ease;
        }

        .quantum-range::-webkit-slider-thumb:hover {
          transform: scale(1.2);
        }

        .value-display {
          margin: 20px 0;
        }

        .value-box {
          background: rgba(0, 0, 0, 0.2);
          padding: 15px;
          border-radius: 8px;
          text-align: center;
        }

        .value-label {
          display: block;
          font-size: 12px;
          opacity: 0.8;
          margin-bottom: 5px;
        }

        .value-number {
          display: block;
          font-size: 32px;
          font-weight: bold;
        }

        .quantum-info {
          background: rgba(0, 0, 0, 0.2);
          padding: 15px;
          border-radius: 8px;
          margin: 20px 0;
        }

        .quantum-info h3 {
          margin-top: 0;
          font-size: 16px;
        }

        .quantum-info ul {
          margin: 10px 0;
          padding-left: 20px;
        }

        .quantum-info li {
          margin: 8px 0;
          font-size: 13px;
          line-height: 1.5;
        }

        .instructions {
          background: rgba(0, 0, 0, 0.2);
          padding: 15px;
          border-radius: 8px;
          margin: 20px 0;
        }

        .instructions h3 {
          margin-top: 0;
          font-size: 16px;
        }

        .instructions ol {
          margin: 10px 0;
          padding-left: 20px;
        }

        .instructions li {
          margin: 8px 0;
          font-size: 13px;
          line-height: 1.6;
        }

        .instructions code {
          background: rgba(255, 255, 255, 0.2);
          padding: 2px 6px;
          border-radius: 3px;
          font-family: 'Consolas', monospace;
          font-size: 12px;
        }

        .debug-info {
          margin-top: 20px;
          background: rgba(0, 0, 0, 0.3);
          padding: 10px;
          border-radius: 8px;
          font-size: 12px;
        }

        .debug-info pre {
          margin: 10px 0 0 0;
          white-space: pre-wrap;
          word-wrap: break-word;
        }
      `}</style>
    </div>
  );
}
