import { useState, useEffect } from '@minimact/core';

interface FileItem {
  name: string;
  path: string;
  type: 'file' | 'directory';
  size: number;
  extension?: string;
  modified: string;
  created: string;
}

interface DirectoryData {
  currentPath: string;
  parentPath: string | null;
  items: FileItem[];
  totalFiles: number;
  totalDirectories: number;
}

interface FileStats {
  totalFiles: number;
  totalSize: number;
  extensionStats: {
    extension: string;
    count: number;
    totalSize: number;
  }[];
}

export function FileManagerPage() {
  const [currentPath, setCurrentPath] = useState<string>('');
  const [directoryData, setDirectoryData] = useState<DirectoryData | null>(null);
  const [fileStats, setFileStats] = useState<FileStats | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<FileItem | null>(null);

  // Load directory on mount and when path changes
  useEffect(() => {
    loadDirectory(currentPath);
  }, [currentPath]);

  async function loadDirectory(path: string) {
    setLoading(true);
    setError(null);

    try {
      // Load directory contents
      const dirResponse = await fetch(`/api/desktop/directory?path=${encodeURIComponent(path)}`);
      if (!dirResponse.ok) {
        throw new Error('Failed to load directory');
      }
      const dirData = await dirResponse.json();
      setDirectoryData(dirData);

      // Load file statistics
      const statsResponse = await fetch(`/api/desktop/file-stats?path=${encodeURIComponent(path)}`);
      if (statsResponse.ok) {
        const stats = await statsResponse.json();
        setFileStats(stats);
      }

      // Update current path
      setCurrentPath(dirData.currentPath);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  function handleItemClick(item: FileItem) {
    if (item.type === 'directory') {
      setCurrentPath(item.path);
      setSelectedFile(null);
    } else {
      setSelectedFile(item);
    }
  }

  function handleGoUp() {
    if (directoryData?.parentPath) {
      setCurrentPath(directoryData.parentPath);
      setSelectedFile(null);
    }
  }

  function formatFileSize(bytes: number): string {
    if (bytes === 0) return '-';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleString();
  }

  // Prepare chart data
  const chartData = fileStats?.extensionStats.map(stat => ({
    name: stat.extension,
    count: stat.count,
    size: stat.totalSize
  })) || [];

  return (
    <div className="file-manager">
      <style>{`
        * {
          margin: 0;
          padding: 0;
          box-sizing: border-box;
        }

        body {
          font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
          background: #1a1a1a;
          color: #e0e0e0;
        }

        .file-manager {
          display: flex;
          flex-direction: column;
          height: 100vh;
          padding: 20px;
          gap: 20px;
        }

        .header {
          background: #2a2a2a;
          padding: 20px;
          border-radius: 12px;
          border: 1px solid #3a3a3a;
        }

        .header h1 {
          font-size: 24px;
          font-weight: 600;
          margin-bottom: 16px;
          color: #fff;
        }

        .path-bar {
          display: flex;
          align-items: center;
          gap: 12px;
          background: #1a1a1a;
          padding: 12px;
          border-radius: 8px;
          border: 1px solid #3a3a3a;
        }

        .path-bar button {
          background: #3a3a3a;
          border: none;
          color: #e0e0e0;
          padding: 8px 16px;
          border-radius: 6px;
          cursor: pointer;
          font-size: 14px;
          transition: background 0.2s;
        }

        .path-bar button:hover {
          background: #4a4a4a;
        }

        .path-bar button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .current-path {
          flex: 1;
          font-family: 'Courier New', monospace;
          font-size: 13px;
          color: #888;
        }

        .stats-summary {
          display: flex;
          gap: 12px;
          margin-top: 12px;
        }

        .stat-item {
          background: #1a1a1a;
          padding: 8px 16px;
          border-radius: 6px;
          border: 1px solid #3a3a3a;
          font-size: 13px;
        }

        .stat-label {
          color: #888;
          margin-right: 8px;
        }

        .stat-value {
          color: #4a9eff;
          font-weight: 600;
        }

        .content {
          display: grid;
          grid-template-columns: 1fr 400px;
          gap: 20px;
          flex: 1;
          min-height: 0;
        }

        .file-list-panel {
          background: #2a2a2a;
          border-radius: 12px;
          border: 1px solid #3a3a3a;
          display: flex;
          flex-direction: column;
          overflow: hidden;
        }

        .file-list {
          flex: 1;
          overflow-y: auto;
          padding: 12px;
        }

        .file-item {
          display: grid;
          grid-template-columns: 40px 1fr 120px 180px;
          gap: 12px;
          padding: 12px;
          border-radius: 8px;
          cursor: pointer;
          transition: background 0.2s;
          align-items: center;
        }

        .file-item:hover {
          background: #3a3a3a;
        }

        .file-item.selected {
          background: #2a4a7a;
        }

        .file-icon {
          width: 32px;
          height: 32px;
          display: flex;
          align-items: center;
          justify-content: center;
          background: #3a3a3a;
          border-radius: 6px;
          font-size: 18px;
        }

        .file-name {
          font-size: 14px;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .file-size {
          font-size: 13px;
          color: #888;
          text-align: right;
        }

        .file-date {
          font-size: 12px;
          color: #666;
          font-family: 'Courier New', monospace;
        }

        .sidebar {
          display: flex;
          flex-direction: column;
          gap: 20px;
        }

        .panel {
          background: #2a2a2a;
          border-radius: 12px;
          border: 1px solid #3a3a3a;
          padding: 20px;
        }

        .panel h2 {
          font-size: 16px;
          font-weight: 600;
          margin-bottom: 16px;
          color: #fff;
        }

        .chart-container {
          min-height: 300px;
        }

        .loading {
          text-align: center;
          padding: 40px;
          color: #888;
        }

        .error {
          background: #4a2a2a;
          border: 1px solid #6a3a3a;
          border-radius: 8px;
          padding: 16px;
          color: #ff6b6b;
        }

        .empty {
          text-align: center;
          padding: 40px;
          color: #666;
        }
      `}</style>

      {/* Header */}
      <div className="header">
        <h1>📁 Minimact File Manager</h1>
        <div className="path-bar">
          <button onClick={handleGoUp} disabled={!directoryData?.parentPath}>
            ⬆️ Up
          </button>
          <span className="current-path">{currentPath || 'Loading...'}</span>
        </div>
        {directoryData && (
          <div className="stats-summary">
            <div className="stat-item">
              <span className="stat-label">Directories:</span>
              <span className="stat-value">{directoryData.totalDirectories}</span>
            </div>
            <div className="stat-item">
              <span className="stat-label">Files:</span>
              <span className="stat-value">{directoryData.totalFiles}</span>
            </div>
            {fileStats && (
              <div className="stat-item">
                <span className="stat-label">Total Size:</span>
                <span className="stat-value">{formatFileSize(fileStats.totalSize)}</span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Content */}
      <div className="content">
        {/* File List */}
        <div className="file-list-panel">
          {loading && <div className="loading">Loading...</div>}
          {error && <div className="error">Error: {error}</div>}
          {!loading && !error && directoryData && (
            <div className="file-list">
              {directoryData.items.length === 0 ? (
                <div className="empty">Empty directory</div>
              ) : (
                directoryData.items.map((item, index) => (
                  <div
                    key={index}
                    className={`file-item ${selectedFile?.path === item.path ? 'selected' : ''}`}
                    onClick={() => handleItemClick(item)}
                  >
                    <div className="file-icon">
                      {item.type === 'directory' ? '📁' : '📄'}
                    </div>
                    <div className="file-name">{item.name}</div>
                    <div className="file-size">{formatFileSize(item.size)}</div>
                    <div className="file-date">{formatDate(item.modified)}</div>
                  </div>
                ))
              )}
            </div>
          )}
        </div>

        {/* Sidebar with Charts */}
        <div className="sidebar">
          {/* File Type Distribution Chart */}
          {fileStats && chartData.length > 0 && (
            <div className="panel">
              <h2>📊 File Types</h2>
              <div className="chart-container">
                <Plugin name="BarChart" state={{
                  data: chartData,
                  width: 360,
                  height: 250,
                  margin: { top: 10, right: 10, bottom: 40, left: 40 },
                  xAxisDataKey: 'name',
                  yAxisDataKey: 'count',
                  barFill: '#4a9eff',
                  backgroundColor: '#1a1a1a',
                  axisColor: '#666',
                  gridColor: '#2a2a2a'
                }} />
              </div>
            </div>
          )}

          {/* Selected File Info */}
          {selectedFile && (
            <div className="panel">
              <h2>📄 File Info</h2>
              <div style={{ fontSize: '13px', lineHeight: '1.8' }}>
                <div><strong>Name:</strong> {selectedFile.name}</div>
                <div><strong>Type:</strong> {selectedFile.extension || 'Unknown'}</div>
                <div><strong>Size:</strong> {formatFileSize(selectedFile.size)}</div>
                <div><strong>Modified:</strong> {formatDate(selectedFile.modified)}</div>
                <div style={{ marginTop: '12px', fontSize: '11px', color: '#666', wordBreak: 'break-all' }}>
                  {selectedFile.path}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
