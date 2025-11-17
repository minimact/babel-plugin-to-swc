/**
 * PricingCalculator Component - Fixture for Ruby Ranger
 *
 * Tests useDecisionTree from minimact-trees extension
 *
 * Features tested:
 * - Nested decision paths
 * - Universal value type support (numbers, strings, objects)
 * - Context-based tree navigation
 * - Predictive transition pre-computation
 * - TypeScript type inference
 */

import { useDecisionTree } from 'minimact-trees';
import { useState } from '@minimact/core';

export function PricingCalculator() {
  const [role, setRole] = useState('basic');
  const [itemCount, setItemCount] = useState(1);
  const [region, setRegion] = useState('domestic');

  // Decision tree for shipping price calculation
  const shippingPrice = useDecisionTree({
    // Admin users always get free shipping
    roleAdmin: 0,

    // Premium users get conditional free shipping
    rolePremium: {
      count5: 0,      // 5+ items = free
      count3: 5,      // 3-4 items = $5
      count1: 10      // 1-2 items = $10
    },

    // Basic users pay based on region
    roleBasic: {
      regionDomestic: {
        count5: 5,
        count3: 10,
        count1: 15
      },
      regionInternational: {
        count5: 25,
        count3: 35,
        count1: 50
      }
    }
  }, {
    role: role,
    count: itemCount,
    region: region
  });

  // Decision tree for discount calculation
  const discountPercent = useDecisionTree({
    roleAdmin: 50,
    rolePremium: 20,
    roleBasic: 0
  }, {
    role: role
  });

  // Calculate totals
  const subtotal = itemCount * 100; // $100 per item
  const discount = Math.round(subtotal * (discountPercent / 100));
  const total = subtotal - discount + shippingPrice;

  return (
    <div className="pricing-calculator">
      <h1>Pricing Calculator</h1>

      <div className="controls">
        <div className="control-group">
          <label>Role:</label>
          <select value={role} onChange={(e) => setRole(e.target.value)}>
            <option value="basic">Basic</option>
            <option value="premium">Premium</option>
            <option value="admin">Admin</option>
          </select>
        </div>

        <div className="control-group">
          <label>Item Count:</label>
          <input
            type="number"
            value={itemCount}
            onChange={(e) => setItemCount(parseInt(e.target.value) || 1)}
            min="1"
            max="10"
          />
        </div>

        <div className="control-group">
          <label>Region:</label>
          <select value={region} onChange={(e) => setRegion(e.target.value)}>
            <option value="domestic">Domestic</option>
            <option value="international">International</option>
          </select>
        </div>
      </div>

      <div className="results">
        <div className="result-row">
          <span>Subtotal:</span>
          <span className="amount">${subtotal}</span>
        </div>

        <div className="result-row">
          <span>Discount ({discountPercent}%):</span>
          <span className="amount discount">-${discount}</span>
        </div>

        <div className="result-row">
          <span>Shipping:</span>
          <span className="amount">${shippingPrice}</span>
        </div>

        <div className="result-row total">
          <span>Total:</span>
          <span className="amount">${total}</span>
        </div>
      </div>

      <div className="debug-info">
        <h3>Decision Tree Results:</h3>
        <div>Shipping Price: ${shippingPrice}</div>
        <div>Discount: {discountPercent}%</div>
        <div>Context: {JSON.stringify({ role, count: itemCount, region })}</div>
      </div>
    </div>
  );
}
