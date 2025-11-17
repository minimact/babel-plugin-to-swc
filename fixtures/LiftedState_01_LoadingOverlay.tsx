// Pattern 1.1: Parent Observing Child State - Loading Overlay
// Parent watches multiple child loading states and shows an overlay when ANY child is loading

// Note: <Component> is a special compile-time JSX element recognized by the Babel plugin
// It's not imported - it's part of the Minimact JSX syntax
import { useState } from '@minimact/core';

// Child component: UserProfile
export function UserProfile() {
  const [isLoading, setIsLoading] = useState(false);

  const handleRefresh = () => {
    setIsLoading(true);
    // Note: In real app, this would be an async API call
    // For now, just toggle the state
    setTimeout(() => setIsLoading(false), 2000);
  };

  return (
    <div className="user-profile">
      <h3>User Profile</h3>
      <button id="refresh-profile-btn" type="button" onClick={handleRefresh}>
        Refresh Profile
      </button>
      {isLoading && <span className="loading-indicator">Loading...</span>}
    </div>
  );
}

// Child component: ShoppingCart
export function ShoppingCart() {
  const [isLoading, setIsLoading] = useState(false);

  const handleRefresh = () => {
    setIsLoading(true);
    setTimeout(() => setIsLoading(false), 1500);
  };

  return (
    <div className="shopping-cart">
      <h3>Shopping Cart</h3>
      <button id="refresh-cart-btn" type="button" onClick={handleRefresh}>
        Refresh Cart
      </button>
      {isLoading && <span className="loading-indicator">Loading...</span>}
    </div>
  );
}

// Child component: ContactForm
export function ContactForm() {
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = () => {
    setIsLoading(true);
    setTimeout(() => setIsLoading(false), 1000);
  };

  return (
    <div className="contact-form">
      <h3>Contact Form</h3>
      <button id="submit-form-btn" type="button" onClick={handleSubmit}>
        Submit Form
      </button>
      {isLoading && <span className="loading-indicator">Loading...</span>}
    </div>
  );
}

// Parent component: Dashboard
export default function Dashboard() {
  // Observe child loading states
  const userLoading = state["UserProfile.isLoading"];
  const cartLoading = state["ShoppingCart.isLoading"];
  const formLoading = state["ContactForm.isLoading"];

  const anyLoading = userLoading || cartLoading || formLoading;

  return (
    <div id="dashboard-root">
      <h1>Dashboard</h1>

      {/* Overlay appears when ANY child is loading */}
      {anyLoading && (
        <div id="loading-overlay" className="loading-overlay">
          <div className="spinner"></div>
          <p>Loading...</p>
        </div>
      )}

      <Component name="UserProfile" state={{ isLoading: false }}>
        <UserProfile />
      </Component>

      <Component name="ShoppingCart" state={{ isLoading: false }}>
        <ShoppingCart />
      </Component>

      <Component name="ContactForm" state={{ isLoading: false }}>
        <ContactForm />
      </Component>

      {/* Status display for testing */}
      <div id="status" className="status">
        <p>User Loading: <span id="user-loading">{userLoading ? 'Yes' : 'No'}</span></p>
        <p>Cart Loading: <span id="cart-loading">{cartLoading ? 'Yes' : 'No'}</span></p>
        <p>Form Loading: <span id="form-loading">{formLoading ? 'Yes' : 'No'}</span></p>
        <p>Any Loading: <span id="any-loading">{anyLoading ? 'Yes' : 'No'}</span></p>
      </div>
    </div>
  );
}
