import { useServerTask, useState } from '@minimact/core';

interface UploadResult {
  success: boolean;
  url?: string;
  fileSize?: number;
  error?: string;
}

export function FileUpload() {
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  // Long-running server task with progress tracking
  const uploadTask = useServerTask<UploadResult>(
    async (file: File) => {
      // SERVER-SIDE CODE (transpiled to C#, runs in ASP.NET Core)

      console.log(`Starting upload of ${file.name} (${file.size} bytes)`);

      // Simulate file processing with progress updates
      for (let progress = 0; progress <= 100; progress += 10) {
        await Task.Delay(200); // Server-side delay (C#)

        // Update progress (triggers client update via SignalR)
        uploadTask.updateProgress(progress / 100);

        console.log(`Upload progress: ${progress}%`);
      }

      // Simulate file storage
      const fileUrl = `/uploads/${Date.now()}_${file.name}`;

      console.log(`Upload completed: ${fileUrl}`);

      return {
        success: true,
        url: fileUrl,
        fileSize: file.size
      };
    },
    {
      runtime: 'csharp' // Execute on C# runtime (can also be 'rust' for Rayon parallelism)
    }
  );

  return (
    <div className="file-upload">
      <h1>File Upload Demo</h1>
      <p>Test long-running server tasks with real-time progress</p>

      <div className="upload-controls">
        <input
          type="file"
          onChange={(e) => setSelectedFile(e.target.files?.[0] || null)}
        />

        <button
          onClick={() => uploadTask.start(selectedFile!)}
          disabled={!selectedFile || uploadTask.status === 'running'}
        >
          {uploadTask.status === 'running' ? 'Uploading...' : 'Upload File'}
        </button>
      </div>

      {/* Status Display */}
      <div className="status">
        {uploadTask.status === 'idle' && (
          <div className="status-idle">Select a file to upload</div>
        )}

        {uploadTask.status === 'running' && (
          <div className="status-running">
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${uploadTask.progress * 100}%` }}
              />
            </div>
            <p>Uploading... {Math.round(uploadTask.progress * 100)}%</p>
            <button onClick={() => uploadTask.cancel()}>Cancel</button>
          </div>
        )}

        {uploadTask.status === 'success' && uploadTask.data && (
          <div className="status-success">
            <p>✅ Upload successful!</p>
            <p>File URL: {uploadTask.data.url}</p>
            <p>File Size: {uploadTask.data.fileSize} bytes</p>
            <button onClick={() => uploadTask.start(selectedFile!)}>
              Upload Again
            </button>
          </div>
        )}

        {uploadTask.status === 'error' && (
          <div className="status-error">
            <p>❌ Upload failed: {uploadTask.error}</p>
            <button onClick={() => uploadTask.retry(selectedFile!)}>
              Retry Upload
            </button>
          </div>
        )}
      </div>

      {/* Task State Debug Info */}
      <div className="debug-info">
        <h3>Task State (Debug)</h3>
        <pre>
          {JSON.stringify({
            status: uploadTask.status,
            progress: uploadTask.progress,
            hasData: !!uploadTask.data,
            hasError: !!uploadTask.error
          }, null, 2)}
        </pre>
      </div>
    </div>
  );
}
