import { useServerTask, useState } from '@minimact/core';

interface ProcessingResult {
  totalProcessed: number;
  averageValue: number;
  maxValue: number;
  minValue: number;
  processingTimeMs: number;
  itemsPerSecond: number;
}

export function ParallelDataProcessing() {
  const [dataSize, setDataSize] = useState<number>(10000);

  // Rust-powered parallel data processing with Rayon
  // This showcases the benefits of Rust's parallel iterators for CPU-intensive tasks
  const processDataRust = useServerTask<ProcessingResult>(
    async (count: number) => {
      // SERVER-SIDE CODE (transpiled to Rust, runs with Rayon parallelism)

      console.log(`Starting Rust parallel processing of ${count} items...`);
      const startTime = Date.now();

      // Generate large dataset
      let data: number[] = [];
      for (let i = 0; i < count; i++) {
        data.push(Math.random() * 1000);
      }

      // ✨ Rayon parallel processing (automatically parallelized in Rust)
      // This becomes: data.par_iter().map(|x| expensive_calculation(x))
      let processed = data.map(value => {
        // Expensive calculation (simulated)
        let result = value;
        for (let j = 0; j < 1000; j++) {
          result = Math.sqrt(result * result + 1);
        }
        return result;
      });

      // Aggregate results (also parallelized with Rayon)
      let sum = 0;
      let max = -Infinity;
      let min = Infinity;

      for (let value of processed) {
        sum += value;
        if (value > max) max = value;
        if (value < min) min = value;

        // Update progress periodically
        if (processed.indexOf(value) % 1000 === 0) {
          processDataRust.updateProgress(processed.indexOf(value) / count);
        }
      }

      const endTime = Date.now();
      const processingTime = endTime - startTime;

      console.log(`Rust processing completed in ${processingTime}ms`);

      return {
        totalProcessed: count,
        averageValue: sum / count,
        maxValue: max,
        minValue: min,
        processingTimeMs: processingTime,
        itemsPerSecond: Math.round(count / (processingTime / 1000))
      };
    },
    {
      runtime: 'rust' // 🦀 Execute on Rust runtime with Rayon parallelism
    }
  );

  // C# version for comparison (sequential processing)
  const processDataCSharp = useServerTask<ProcessingResult>(
    async (count: number) => {
      // SERVER-SIDE CODE (transpiled to C#, runs sequentially)

      console.log(`Starting C# sequential processing of ${count} items...`);
      const startTime = Date.now();

      // Generate large dataset
      let data: number[] = [];
      for (let i = 0; i < count; i++) {
        data.push(Math.random() * 1000);
      }

      // Sequential processing
      let processed = data.map(value => {
        // Expensive calculation (simulated)
        let result = value;
        for (let j = 0; j < 1000; j++) {
          result = Math.sqrt(result * result + 1);
        }
        return result;
      });

      // Aggregate results
      let sum = 0;
      let max = -Infinity;
      let min = Infinity;

      for (let value of processed) {
        sum += value;
        if (value > max) max = value;
        if (value < min) min = value;

        // Update progress periodically
        if (processed.indexOf(value) % 1000 === 0) {
          processDataCSharp.updateProgress(processed.indexOf(value) / count);
        }
      }

      const endTime = Date.now();
      const processingTime = endTime - startTime;

      console.log(`C# processing completed in ${processingTime}ms`);

      return {
        totalProcessed: count,
        averageValue: sum / count,
        maxValue: max,
        minValue: min,
        processingTimeMs: processingTime,
        itemsPerSecond: Math.round(count / (processingTime / 1000))
      };
    },
    {
      runtime: 'csharp' // C# runtime (sequential)
    }
  );

  return (
    <div className="parallel-processing">
      <h1>🦀 Rust vs C# Performance Comparison</h1>
      <p>Compare Rust's Rayon parallel processing vs C# sequential processing</p>

      {/* Data Size Controls */}
      <div className="controls">
        <label>
          Dataset Size:
          <input
            type="number"
            value={dataSize}
            onChange={(e) => setDataSize(parseInt(e.target.value) || 1000)}
            min="1000"
            max="100000"
            step="1000"
          />
        </label>
        <p className="hint">
          {dataSize < 10000 ? 'Small dataset' :
           dataSize < 50000 ? 'Medium dataset' :
           'Large dataset - Rust will shine!'}
        </p>
      </div>

      {/* Rust Processing */}
      <div className="rust-section">
        <h2>🦀 Rust + Rayon (Parallel)</h2>

        <button
          onClick={() => processDataRust.start(dataSize)}
          disabled={processDataRust.status === 'running'}
          className="btn-rust"
        >
          {processDataRust.status === 'running' ? 'Processing...' : 'Run Rust Version'}
        </button>

        {processDataRust.status === 'running' && (
          <div className="progress">
            <div className="progress-bar">
              <div
                className="progress-fill rust"
                style={{ width: `${processDataRust.progress * 100}%` }}
              />
            </div>
            <p>{Math.round(processDataRust.progress * 100)}% complete</p>
            <button onClick={() => processDataRust.cancel()}>Cancel</button>
          </div>
        )}

        {processDataRust.status === 'success' && processDataRust.data && (
          <div className="results rust-results">
            <h3>✅ Rust Results</h3>
            <table>
              <tbody>
                <tr>
                  <td>Items Processed:</td>
                  <td><strong>{processDataRust.data.totalProcessed.toLocaleString()}</strong></td>
                </tr>
                <tr>
                  <td>Processing Time:</td>
                  <td><strong>{processDataRust.data.processingTimeMs}ms</strong></td>
                </tr>
                <tr>
                  <td>Throughput:</td>
                  <td><strong>{processDataRust.data.itemsPerSecond.toLocaleString()} items/sec</strong></td>
                </tr>
                <tr>
                  <td>Average Value:</td>
                  <td>{processDataRust.data.averageValue.toFixed(2)}</td>
                </tr>
                <tr>
                  <td>Min / Max:</td>
                  <td>{processDataRust.data.minValue.toFixed(2)} / {processDataRust.data.maxValue.toFixed(2)}</td>
                </tr>
              </tbody>
            </table>
          </div>
        )}

        {processDataRust.status === 'error' && (
          <div className="error">
            <p>❌ Error: {processDataRust.error}</p>
            <button onClick={() => processDataRust.retry(dataSize)}>Retry</button>
          </div>
        )}
      </div>

      {/* C# Processing */}
      <div className="csharp-section">
        <h2>🔷 C# (Sequential)</h2>

        <button
          onClick={() => processDataCSharp.start(dataSize)}
          disabled={processDataCSharp.status === 'running'}
          className="btn-csharp"
        >
          {processDataCSharp.status === 'running' ? 'Processing...' : 'Run C# Version'}
        </button>

        {processDataCSharp.status === 'running' && (
          <div className="progress">
            <div className="progress-bar">
              <div
                className="progress-fill csharp"
                style={{ width: `${processDataCSharp.progress * 100}%` }}
              />
            </div>
            <p>{Math.round(processDataCSharp.progress * 100)}% complete</p>
            <button onClick={() => processDataCSharp.cancel()}>Cancel</button>
          </div>
        )}

        {processDataCSharp.status === 'success' && processDataCSharp.data && (
          <div className="results csharp-results">
            <h3>✅ C# Results</h3>
            <table>
              <tbody>
                <tr>
                  <td>Items Processed:</td>
                  <td><strong>{processDataCSharp.data.totalProcessed.toLocaleString()}</strong></td>
                </tr>
                <tr>
                  <td>Processing Time:</td>
                  <td><strong>{processDataCSharp.data.processingTimeMs}ms</strong></td>
                </tr>
                <tr>
                  <td>Throughput:</td>
                  <td><strong>{processDataCSharp.data.itemsPerSecond.toLocaleString()} items/sec</strong></td>
                </tr>
                <tr>
                  <td>Average Value:</td>
                  <td>{processDataCSharp.data.averageValue.toFixed(2)}</td>
                </tr>
                <tr>
                  <td>Min / Max:</td>
                  <td>{processDataCSharp.data.minValue.toFixed(2)} / {processDataCSharp.data.maxValue.toFixed(2)}</td>
                </tr>
              </tbody>
            </table>
          </div>
        )}

        {processDataCSharp.status === 'error' && (
          <div className="error">
            <p>❌ Error: {processDataCSharp.error}</p>
            <button onClick={() => processDataCSharp.retry(dataSize)}>Retry</button>
          </div>
        )}
      </div>

      {/* Performance Comparison */}
      {processDataRust.status === 'success' &&
       processDataCSharp.status === 'success' &&
       processDataRust.data &&
       processDataCSharp.data && (
        <div className="comparison">
          <h2>📊 Performance Comparison</h2>
          <table className="comparison-table">
            <thead>
              <tr>
                <th>Metric</th>
                <th>Rust 🦀</th>
                <th>C# 🔷</th>
                <th>Speedup</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Processing Time</td>
                <td>{processDataRust.data.processingTimeMs}ms</td>
                <td>{processDataCSharp.data.processingTimeMs}ms</td>
                <td className="speedup">
                  <strong>
                    {(processDataCSharp.data.processingTimeMs / processDataRust.data.processingTimeMs).toFixed(2)}x faster
                  </strong>
                </td>
              </tr>
              <tr>
                <td>Throughput</td>
                <td>{processDataRust.data.itemsPerSecond.toLocaleString()} items/sec</td>
                <td>{processDataCSharp.data.itemsPerSecond.toLocaleString()} items/sec</td>
                <td className="speedup">
                  <strong>
                    {(processDataRust.data.itemsPerSecond / processDataCSharp.data.itemsPerSecond).toFixed(2)}x faster
                  </strong>
                </td>
              </tr>
            </tbody>
          </table>

          <div className="insight">
            <p>
              💡 <strong>Insight:</strong> Rust with Rayon automatically parallelizes the computation across all CPU cores,
              while C# runs sequentially. For large datasets and CPU-intensive tasks, Rust can be{' '}
              <strong>{(processDataCSharp.data.processingTimeMs / processDataRust.data.processingTimeMs).toFixed(1)}x faster</strong>!
            </p>
          </div>
        </div>
      )}

      {/* Debug Info */}
      <details className="debug-info">
        <summary>Task State (Debug)</summary>
        <div className="debug-grid">
          <div>
            <h4>Rust Task</h4>
            <pre>
              {JSON.stringify({
                status: processDataRust.status,
                progress: processDataRust.progress,
                hasData: !!processDataRust.data,
                hasError: !!processDataRust.error
              }, null, 2)}
            </pre>
          </div>
          <div>
            <h4>C# Task</h4>
            <pre>
              {JSON.stringify({
                status: processDataCSharp.status,
                progress: processDataCSharp.progress,
                hasData: !!processDataCSharp.data,
                hasError: !!processDataCSharp.error
              }, null, 2)}
            </pre>
          </div>
        </div>
      </details>
    </div>
  );
}
