// Pattern 5.1: Complex Workflow Orchestration - Email Composer with Rules
// Parent enforces business rules and coordinates complex multi-component workflow

import { Component, state, setState } from '@minimact/core';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
}

// Child component: RecipientList
export function RecipientList() {
  const recipients = state.recipients || [];

  const handleAdd = () => {
    const email = prompt('Enter recipient email:');
    if (email && email.includes('@')) {
      setState('recipients', [...recipients, email]);
    }
  };

  const handleRemove = (index: number) => {
    setState('recipients', recipients.filter((_: string, i: number) => i !== index));
  };

  return (
    <div className="recipient-list">
      <h3>To:</h3>
      <div id="recipients">
        {recipients.map((email: string, idx: number) => (
          <div key={idx} className="recipient-chip" data-recipient-index={idx}>
            {email}
            <button
              type="button"
              className="remove-recipient-btn"
              onClick={() => handleRemove(idx)}
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <button
        id="add-recipient-btn"
        type="button"
        onClick={handleAdd}
      >
        Add Recipient
      </button>
    </div>
  );
}

// Child component: SubjectLine
export function SubjectLine() {
  const text = state.text;

  return (
    <div className="subject-line">
      <label htmlFor="subject-input">Subject:</label>
      <input
        id="subject-input"
        type="text"
        value={text}
        onInput={(e) => setState('text', e.target.value)}
        placeholder="Enter subject..."
      />
    </div>
  );
}

// Child component: MessageBody
export function MessageBody() {
  const content = state.content;

  return (
    <div className="message-body">
      <label htmlFor="body-textarea">Message:</label>
      <textarea
        id="body-textarea"
        value={content}
        onInput={(e) => setState('content', e.target.value)}
        placeholder="Enter message..."
        rows={10}
      />
      <p>Characters: <span id="body-char-count">{content.length}</span></p>
    </div>
  );
}

// Child component: AttachmentPanel
export function AttachmentPanel({ maxSize }: { maxSize: number }) {
  const files = state.files || [];
  const totalSize = state.totalSize || 0;

  const handleFileAdd = () => {
    // Simulate file addition
    const fileSize = Math.floor(Math.random() * 5000000) + 1000000; // 1-6 MB
    const fileName = `file_${Date.now()}.pdf`;

    const newFiles = [...files, { name: fileName, size: fileSize }];
    const newSize = newFiles.reduce((sum: number, f: any) => sum + f.size, 0);

    if (newSize <= maxSize) {
      setState('files', newFiles);
      setState('totalSize', newSize);
    } else {
      alert(`File too large! Would exceed ${formatBytes(maxSize)} limit.`);
    }
  };

  const handleFileRemove = (index: number) => {
    const newFiles = files.filter((_: any, i: number) => i !== index);
    const newSize = newFiles.reduce((sum: number, f: any) => sum + f.size, 0);
    setState('files', newFiles);
    setState('totalSize', newSize);
  };

  return (
    <div className="attachment-panel">
      <h3>Attachments (<span id="attachment-size">{formatBytes(totalSize)}</span> / {formatBytes(maxSize)})</h3>
      <button
        id="add-file-btn"
        type="button"
        onClick={handleFileAdd}
      >
        Add File (Simulated)
      </button>
      <ul id="file-list">
        {files.map((file: any, idx: number) => (
          <li key={idx} data-file-index={idx}>
            {file.name} ({formatBytes(file.size)})
            <button
              type="button"
              className="remove-file-btn"
              onClick={() => handleFileRemove(idx)}
            >
              Remove
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

// Parent component: EmailComposer
export default function EmailComposer() {
  // Observe all child states
  const recipients = state["RecipientList.recipients"] || [];
  const subject = state["SubjectLine.text"] || "";
  const body = state["MessageBody.content"] || "";
  const attachments = state["AttachmentPanel.files"] || [];
  const totalSize = state["AttachmentPanel.totalSize"] || 0;

  // Business rules
  const MAX_ATTACHMENT_SIZE = 25 * 1024 * 1024; // 25MB
  const MIN_RECIPIENTS = 1;
  const MIN_BODY_LENGTH = 10;

  // Validation
  const hasRecipients = recipients.length >= MIN_RECIPIENTS;
  const hasSubject = subject.length > 0;
  const hasBody = body.length >= MIN_BODY_LENGTH;
  const attachmentsValid = totalSize <= MAX_ATTACHMENT_SIZE;

  const canSend = hasRecipients && hasSubject && hasBody && attachmentsValid;

  // Validation messages
  const errors = [
    !hasRecipients && "Add at least one recipient",
    !hasSubject && "Enter a subject",
    !hasBody && `Message body must be at least ${MIN_BODY_LENGTH} characters`,
    !attachmentsValid && `Attachments too large (${formatBytes(totalSize)} / ${formatBytes(MAX_ATTACHMENT_SIZE)})`
  ].filter(Boolean);

  const handleSend = () => {
    if (canSend) {
      const email = { recipients, subject, body, attachments };
      console.log('Sending email:', email);
      alert('Email sent successfully!');

      // Reset all components after send
      setState("RecipientList.recipients", []);
      setState("SubjectLine.text", "");
      setState("MessageBody.content", "");
      setState("AttachmentPanel.files", []);
      setState("AttachmentPanel.totalSize", 0);
    }
  };

  const handleClearAttachments = () => {
    setState("AttachmentPanel.files", []);
    setState("AttachmentPanel.totalSize", 0);
  };

  const handleReset = () => {
    setState("RecipientList.recipients", []);
    setState("SubjectLine.text", "");
    setState("MessageBody.content", "");
    setState("AttachmentPanel.files", []);
    setState("AttachmentPanel.totalSize", 0);
  };

  return (
    <div id="email-composer-root" className="email-composer">
      <h1>New Message</h1>

      {/* Validation errors */}
      {errors.length > 0 && (
        <div id="error-panel" className="error-panel">
          <strong>Cannot send:</strong>
          <ul>
            {errors.map((error, idx) => (
              <li key={idx}>{error}</li>
            ))}
          </ul>
        </div>
      )}

      {/* Attachment size warning */}
      {totalSize > MAX_ATTACHMENT_SIZE * 0.8 && (
        <div id="warning-panel" className="warning-panel">
          Approaching attachment size limit ({formatBytes(totalSize)} / {formatBytes(MAX_ATTACHMENT_SIZE)})
          <button
            id="clear-attachments-btn"
            type="button"
            onClick={handleClearAttachments}
          >
            Clear Attachments
          </button>
        </div>
      )}

      <Component name="RecipientList" state={{ recipients: [] }}>
        <RecipientList />
      </Component>

      <Component name="SubjectLine" state={{ text: "" }}>
        <SubjectLine />
      </Component>

      <Component name="MessageBody" state={{ content: "" }}>
        <MessageBody />
      </Component>

      <Component name="AttachmentPanel" state={{ files: [], totalSize: 0 }}>
        <AttachmentPanel maxSize={MAX_ATTACHMENT_SIZE} />
      </Component>

      {/* Send controls */}
      <div className="send-controls">
        <button
          id="send-btn"
          className="btn-send"
          type="button"
          onClick={handleSend}
          disabled={!canSend}
        >
          Send Email
        </button>
        <button
          id="reset-btn"
          type="button"
          onClick={handleReset}
        >
          Reset All
        </button>
        <span id="recipient-count" className="recipient-count">
          To: {recipients.length} recipient{recipients.length !== 1 ? 's' : ''}
        </span>
      </div>

      {/* Status display for testing */}
      <div id="status" className="status">
        <p>Recipients: <span id="status-recipients">{recipients.length}</span></p>
        <p>Subject Length: <span id="status-subject">{subject.length}</span></p>
        <p>Body Length: <span id="status-body">{body.length}</span></p>
        <p>Attachments: <span id="status-attachments">{attachments.length}</span></p>
        <p>Total Size: <span id="status-size">{formatBytes(totalSize)}</span></p>
        <p>Can Send: <span id="status-can-send">{canSend ? 'Yes' : 'No'}</span></p>
      </div>
    </div>
  );
}
