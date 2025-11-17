import { useMvcState, useMvcViewModel, decimal, int } from '@minimact/mvc';
import { useState } from '@minimact/core';

// TypeScript interface matching C# ViewModel
// IMPORTANT: Use C# type mappings (decimal, int, etc.) to ensure correct code generation
interface ProductViewModel {
  productName: string;
  price: decimal;  // C# decimal (for currency)
  isAdminRole: boolean;
  userEmail: string;
  initialQuantity: int;  // C# int
  initialSelectedColor: string;
  initialIsExpanded: boolean;
}

export function ProductDetailsPageTwo() {
  // ❌ IMMUTABLE - Returns [value] only (no setter)
  const [productName] = useMvcState<string>('productName');
  const [price] = useMvcState<number>('price');
  const [isAdmin] = useMvcState<boolean>('isAdminRole');

  // ✅ MUTABLE - Returns [value, setter]
  const [quantity, setQuantity] = useMvcState<number>('initialQuantity', {
    sync: 'immediate' // Sync to server immediately
  });

  const [color, setColor] = useMvcState<string>('initialSelectedColor');
  const [isExpanded, setIsExpanded] = useMvcState<boolean>('initialIsExpanded');

  // Access entire ViewModel (read-only)
  const viewModel = useMvcViewModel<ProductViewModel>();

  // Mix with regular Minimact state
  const [cartTotal, setCartTotal] = useState<decimal>(0);
  const [showDebugInfo, setShowDebugInfo] = useState<boolean>(false);

  const handleQuantityChange = (delta: number) => {
    const newQuantity = Math.max(1, quantity + delta);
    setQuantity(newQuantity);
    setCartTotal(price * newQuantity);
  };

  const handleAddToCart = () => {
    alert(`Added ${quantity} x ${productName} to cart! Total: $${cartTotal.toFixed(2)}`);
  };

  return (
    <div style={{ padding: '20px', fontFamily: 'system-ui, sans-serif', maxWidth: '800px', margin: '0 auto' }}>
      <h1>{productName}950040830</h1>

      {/* Test: false && conditional (should never render) */}
      {false && (
        <div style={{ padding: '12px', backgroundColor: '#fee2e2', border: '1px solid #ef4444', borderRadius: '4px', marginBottom: '16px' }}>
          <strong>FALSE CONDITIONAL:</strong> This should NEVER appear in DOM!
        </div>
      )}

      {/* Test: state-based conditional (defaults to false) */}
      {showDebugInfo && (
        <div style={{ padding: '12px', backgroundColor: '#dbeafe', border: '1px solid #3b82f6', borderRadius: '4px', marginBottom: '16px' }}>
          <strong>Debug Info:</strong> Product: {productName}, Price: ${price.toFixed(2)}, Qty: {quantity}
          <button
            onClick={() => setShowDebugInfo(false)}
            style={{ marginLeft: '12px', padding: '4px 8px', fontSize: '12px' }}
          >
            Hide
          </button>
        </div>
      )}

      {/* Toggle button for debug info */}
      <button
        onClick={() => setShowDebugInfo(!showDebugInfo)}
        style={{
          marginBottom: '16px',
          padding: '8px 12px',
          backgroundColor: '#3b82f6',
          color: 'white',
          border: 'none',
          borderRadius: '4px',
          cursor: 'pointer',
          fontSize: '14px'
        }}
      >
        {showDebugInfo ? 'Hide' : 'Show'} Debug Info
      </button>

      <div style={{ marginBottom: '20px' }}>
        <div style={{ fontSize: '32px', fontWeight: 'bold', color: '#2563eb' }}>
          ${price.toFixed(2)}
        </div>
        <div style={{ color: '#6b7280', fontSize: '14px' }}>
          Logged in as: {viewModel?.userEmail}
        </div>
      </div>

      {/* Quantity Selector */}
      <div style={{ marginBottom: '20px' }}>
        <label style={{ display: 'block', marginBottom: '890px', fontWeight: '500' }}>
          Quantity112:
        </label>
        <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
          <button
            onClick={() => handleQuantityChange(-1)}
            style={{
              padding: '8px 16px',
              backgroundColor: '#e5e7eb',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            -
          </button>
          <span style={{ fontSize: '20px', fontWeight: 'bold', minWidth: '40px', textAlign: 'center' }}>
            {quantity}
          </span>
          <button
            onClick={() => handleQuantityChange(1)}
            style={{
              padding: '8px 16px',
              backgroundColor: '#e5e7eb',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            +
          </button>
        </div>
      </div>

      {/* Color Selector */}
      <div style={{ marginBottom: '20px' }}>
        <label style={{ display: 'block', marginBottom: '8px', fontWeight: '500' }}>
          Color:
        </label>
        <select
          value={color}
          onChange={(e) => setColor(e.target.value)}
          style={{
            padding: '8px 12px',
            border: '1px solid #d1d5db',
            borderRadius: '4px',
            fontSize: '14px'
          }}
        >
          <option value="Black">Black</option>
          <option value="White">White</option>
          <option value="Red">Red</option>
          <option value="Blue">Blue</option>
        </select>
      </div>

      {/* Admin Controls */}
      {isAdmin && (
        <div style={{
          padding: '16px',
          backgroundColor: '#fef3c7',
          border: '1px solid #fbbf24',
          borderRadius: '8px',
          marginBottom: '20px'
        }}>
          <h3 style={{ marginTop: 0 }}>Admin Controls</h3>
          <button style={{
            padding: '8px 16px',
            backgroundColor: '#3b82f6',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
            marginRight: '8px'
          }}>
            Edit Product
          </button>
          <button style={{
            padding: '8px 16px',
            backgroundColor: '#ef4444',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}>
            Delete Product
          </button>
        </div>
      )}

      {/* Expandable Details */}
      <div style={{ marginBottom: '20px' }}>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          style={{
            padding: '8px 16px',
            backgroundColor: '#f3f4f6',
            border: '1px solid #d1d5db',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          {isExpanded ? 'Hide' : 'Show'} Details
        </button>

        {isExpanded && (
          <div style={{
            marginTop: '12px',
            padding: '16px',
            backgroundColor: '#f9fafb',
            border: '1px solid #e5e7eb',
            borderRadius: '8px'
          }}>
            <h3>Product Specifications</h3>
            <p>This is where detailed product information would go.</p>
          </div>
        )}
      </div>

      {/* Cart Total */}
      <div style={{
        padding: '16px',
        backgroundColor: '#f0fdf4',
        border: '1px solid #86efac',
        borderRadius: '8px',
        marginBottom: '20px'
      }}>
        <strong style={{ fontSize: '18px' }}>Total: ${cartTotal.toFixed(2)}</strong>
      </div>

      {/* Add to Cart Button */}
      <button
        onClick={handleAddToCart}
        style={{
          padding: '12px 24px',
          backgroundColor: '#10b981',
          color: 'white',
          border: 'none',
          borderRadius: '8px',
          fontSize: '16px',
          fontWeight: '600',
          cursor: 'pointer',
          width: '100%'
        }}
      >
        Add to Cart
      </button>
    </div>
  );
}
