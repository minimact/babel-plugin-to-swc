/**
 * ProductCard - Test fixture for useDynamicState (Onyx Ranger)
 *
 * Tests minimact-dynamic: Function-based value bindings with dependency tracking
 *
 * Philosophy: Structure ONCE. Bind SEPARATELY. Update DIRECTLY.
 */

import React from 'react';
import { useDynamicState } from 'minimact-dynamic';

interface ProductState {
  user: {
    isPremium: boolean;
    role: 'admin' | 'premium' | 'basic';
  };
  product: {
    price: number;
    factoryPrice: number;
    name: string;
    inStock: boolean;
    imageUrl: string;
  };
  ui: {
    showDetails: boolean;
    theme: 'light' | 'dark';
  };
}

export function ProductCard() {
  // Initialize dynamic state with bindings
  const dynamic = useDynamicState<ProductState>({
    user: {
      isPremium: false,
      role: 'basic'
    },
    product: {
      price: 29.99,
      factoryPrice: 19.99,
      name: 'Cool Gadget',
      inStock: true,
      imageUrl: '/images/gadget.jpg'
    },
    ui: {
      showDetails: false,
      theme: 'light'
    }
  });

  // ========================================
  // Binding Layer (SEPARATE from structure)
  // ========================================

  // Value binding - Price display
  dynamic('.price', (state) =>
    state.user.isPremium
      ? `$${state.product.factoryPrice.toFixed(2)}`
      : `$${state.product.price.toFixed(2)}`
  );

  // Value binding - User badge
  dynamic('.user-badge', (state) =>
    state.user.role.toUpperCase()
  );

  // Value binding - Stock status
  dynamic('.stock-status', (state) =>
    state.product.inStock ? 'In Stock' : 'Out of Stock'
  );

  // Attribute binding - Product image
  dynamic.attr('img.product-image', 'src', (state) =>
    state.product.imageUrl
  );

  // Class binding - Theme and stock
  dynamic.class('.product-card', (state) =>
    `product-card ${state.ui.theme} ${state.product.inStock ? 'in-stock' : 'out-of-stock'}`
  );

  // Style binding - Price color based on role
  dynamic.style('.price', 'color', (state) => {
    if (state.user.role === 'admin') return '#gold';
    if (state.user.role === 'premium') return '#silver';
    return '#black';
  });

  // Visibility binding - Details panel
  dynamic.show('.details-panel', (state) =>
    state.ui.showDetails
  );

  // Visibility binding - Premium badge
  dynamic.show('.premium-badge', (state) =>
    state.user.isPremium
  );

  // ========================================
  // Structure Layer (defined ONCE)
  // ========================================

  return (
    <div className="product-card" data-testid="product-card">
      <img className="product-image" alt="Product" />

      <h2 className="product-name">{dynamic.getState().product.name}</h2>

      <div className="pricing">
        <span className="price" data-testid="price"></span>
        <span className="premium-badge">✨ PREMIUM PRICE</span>
      </div>

      <div className="stock-status" data-testid="stock-status"></div>

      <div className="user-info">
        <span className="user-badge" data-testid="user-badge"></span>
      </div>

      <div className="details-panel" data-testid="details-panel">
        <p>Factory Price: ${dynamic.getState().product.factoryPrice}</p>
        <p>Retail Price: ${dynamic.getState().product.price}</p>
      </div>

      {/* Control buttons for testing */}
      <div className="controls">
        <button
          onClick={() => dynamic.setState({ user: { isPremium: true, role: 'premium' } })}
          data-action="upgrade-premium"
        >
          Upgrade to Premium
        </button>

        <button
          onClick={() => dynamic.setState({ user: { isPremium: false, role: 'basic' } })}
          data-action="downgrade-basic"
        >
          Downgrade to Basic
        </button>

        <button
          onClick={() => dynamic.setState({ user: { role: 'admin' } as any })}
          data-action="set-admin"
        >
          Set Admin
        </button>

        <button
          onClick={() => dynamic.setState({ product: { inStock: !dynamic.getState().product.inStock } as any })}
          data-action="toggle-stock"
        >
          Toggle Stock
        </button>

        <button
          onClick={() => dynamic.setState({ ui: { showDetails: !dynamic.getState().ui.showDetails } as any })}
          data-action="toggle-details"
        >
          Toggle Details
        </button>

        <button
          onClick={() => dynamic.setState({ ui: { theme: dynamic.getState().ui.theme === 'light' ? 'dark' : 'light' } as any })}
          data-action="toggle-theme"
        >
          Toggle Theme
        </button>
      </div>
    </div>
  );
}
