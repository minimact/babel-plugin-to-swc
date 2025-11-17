// Pattern 4.1: Sibling Communication - Shopping Cart Badge
// NavBar reads ShoppingCart state, ProductList modifies it

import { Component, state, setState } from '@minimact/core';

const PRODUCTS = [
  { id: 1, name: "Widget", price: 10.00 },
  { id: 2, name: "Gadget", price: 25.00 },
  { id: 3, name: "Doohickey", price: 15.00 },
  { id: 4, name: "Thingamajig", price: 30.00 }
];

// Child component: NavBar (reads sibling state)
export function NavBar() {
  // Read sibling state directly!
  const cartItems = state["ShoppingCart.items"] || [];
  const cartCount = cartItems.length;

  return (
    <nav className="nav-bar">
      <div className="logo">My Store</div>
      <div id="cart-icon" className="cart-icon">
        🛒
        {cartCount > 0 && (
          <span id="cart-badge" className="badge">{cartCount}</span>
        )}
      </div>
    </nav>
  );
}

// Child component: ProductList (modifies sibling state)
export function ProductList() {
  const handleAddToCart = (product: any) => {
    // Update sibling's state!
    const cartItems = state["ShoppingCart.items"] || [];
    const cartTotal = state["ShoppingCart.total"] || 0;

    setState("ShoppingCart.items", [...cartItems, product]);
    setState("ShoppingCart.total", cartTotal + product.price);
  };

  return (
    <div className="product-list">
      <h2>Products</h2>
      {PRODUCTS.map(product => (
        <div key={product.id} className="product-card">
          <h3>{product.name}</h3>
          <p className="price">${product.price.toFixed(2)}</p>
          <button
            type="button"
            className="add-to-cart-btn"
            data-product-id={product.id}
            onClick={() => handleAddToCart(product)}
          >
            Add to Cart
          </button>
        </div>
      ))}
    </div>
  );
}

// Child component: ShoppingCart
export function ShoppingCart() {
  const items = state.items || [];
  const total = state.total || 0;

  const handleRemoveItem = (index: number) => {
    const itemToRemove = items[index];
    const newItems = items.filter((_: any, i: number) => i !== index);
    const newTotal = total - itemToRemove.price;

    setState('items', newItems);
    setState('total', newTotal);
  };

  const handleClear = () => {
    setState('items', []);
    setState('total', 0);
  };

  return (
    <div className="shopping-cart">
      <h2>Cart</h2>
      {items.length === 0 ? (
        <p id="empty-cart-msg">Cart is empty</p>
      ) : (
        <>
          <div id="cart-items">
            {items.map((item: any, idx: number) => (
              <div key={idx} className="cart-item" data-item-index={idx}>
                <span>{item.name}</span>
                <span>${item.price.toFixed(2)}</span>
                <button
                  type="button"
                  className="remove-item-btn"
                  onClick={() => handleRemoveItem(idx)}
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
          <div id="cart-total" className="total">
            Total: ${total.toFixed(2)}
          </div>
          <button
            id="clear-cart-btn"
            type="button"
            onClick={handleClear}
          >
            Clear Cart
          </button>
        </>
      )}
    </div>
  );
}

// Parent component: ProductPage
export default function ProductPage() {
  const cartItems = state["ShoppingCart.items"] || [];
  const cartTotal = state["ShoppingCart.total"] || 0;

  return (
    <div id="product-page-root">
      {/* Navigation bar with cart count */}
      <Component name="NavBar" state={{}}>
        <NavBar />
      </Component>

      <div className="content">
        {/* Product list that adds to cart */}
        <Component name="ProductList" state={{}}>
          <ProductList />
        </Component>

        {/* Cart component */}
        <Component name="ShoppingCart" state={{ items: [], total: 0 }}>
          <ShoppingCart />
        </Component>
      </div>

      {/* Status display for testing */}
      <div id="status" className="status">
        <p>Cart Items: <span id="cart-item-count">{cartItems.length}</span></p>
        <p>Cart Total: $<span id="cart-total-value">{cartTotal.toFixed(2)}</span></p>
      </div>
    </div>
  );
}
