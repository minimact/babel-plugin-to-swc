// Pattern 3.1: Cross-Component Communication - Chat Application
// Parent coordinates state between MessageInput and MessageList

import { Component, state, setState } from '@minimact/core';

// Child component: ChatHeader
export function ChatHeader() {
  const unreadCount = state.unreadCount;

  return (
    <div className="chat-header">
      <h2>Chat Room</h2>
      {unreadCount > 0 && (
        <span id="unread-badge" className="unread-badge">{unreadCount}</span>
      )}
    </div>
  );
}

// Child component: MessageList
export function MessageList() {
  const messages = state.messages || [];

  return (
    <div className="message-list">
      <h3>Messages ({messages.length})</h3>
      <div id="messages-container">
        {messages.length === 0 ? (
          <p id="no-messages">No messages yet. Start the conversation!</p>
        ) : (
          messages.map(msg => (
            <div key={msg.id} className="message" data-message-id={msg.id}>
              <strong>{msg.author}:</strong> {msg.text}
              <span className="timestamp">{msg.timestamp}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// Child component: MessageInput
export function MessageInput({ onSend }: { onSend: () => void }) {
  const draft = state.draft;

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      onSend();
    }
  };

  return (
    <div className="message-input">
      <textarea
        id="message-textarea"
        value={draft}
        onInput={(e) => setState('draft', e.target.value)}
        onKeyPress={handleKeyPress}
        placeholder="Type a message..."
      />
      <button
        id="send-btn"
        type="button"
        onClick={onSend}
        disabled={!draft.trim()}
      >
        Send
      </button>
      <p>Characters: <span id="char-count">{draft.length}</span></p>
    </div>
  );
}

// Parent component: ChatPage
export default function ChatPage() {
  const draft = state["MessageInput.draft"];
  const messages = state["MessageList.messages"] || [];
  const isTyping = draft.length > 0;

  const handleSend = () => {
    if (draft.trim()) {
      // Add message to list
      const newMessage = {
        id: Date.now(),
        text: draft,
        author: "Me",
        timestamp: new Date().toLocaleTimeString()
      };

      setState("MessageList.messages", [...messages, newMessage]);

      // Clear input
      setState("MessageInput.draft", "");

      // Update unread count badge
      setState("ChatHeader.unreadCount", 0);
    }
  };

  const handleAddBotMessage = () => {
    const botMessage = {
      id: Date.now(),
      text: "This is a bot message",
      author: "Bot",
      timestamp: new Date().toLocaleTimeString()
    };

    setState("MessageList.messages", [...messages, botMessage]);
    setState("ChatHeader.unreadCount", (state["ChatHeader.unreadCount"] || 0) + 1);
  };

  const handleClear = () => {
    setState("MessageList.messages", []);
    setState("MessageInput.draft", "");
    setState("ChatHeader.unreadCount", 0);
  };

  return (
    <div id="chat-root">
      <Component name="ChatHeader" state={{ unreadCount: 0 }}>
        <ChatHeader />
      </Component>

      <Component name="MessageList" state={{ messages: [] }}>
        <MessageList />
      </Component>

      {/* Typing indicator based on input state */}
      {isTyping && (
        <div id="typing-indicator" className="typing-indicator">
          You are typing...
        </div>
      )}

      <Component name="MessageInput" state={{ draft: "" }}>
        <MessageInput onSend={handleSend} />
      </Component>

      {/* Parent controls */}
      <div className="controls">
        <button
          id="add-bot-btn"
          type="button"
          onClick={handleAddBotMessage}
        >
          Add Bot Message
        </button>
        <button
          id="clear-btn"
          type="button"
          onClick={handleClear}
          disabled={messages.length === 0 && draft === ""}
        >
          Clear All
        </button>
      </div>

      {/* Status display for testing */}
      <div id="status" className="status">
        <p>Message Count: <span id="message-count">{messages.length}</span></p>
        <p>Draft Length: <span id="draft-length">{draft.length}</span></p>
        <p>Is Typing: <span id="is-typing">{isTyping ? 'Yes' : 'No'}</span></p>
        <p>Unread: <span id="unread-count">{state["ChatHeader.unreadCount"] || 0}</span></p>
      </div>
    </div>
  );
}
