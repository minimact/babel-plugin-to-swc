import { useState } from '@minimact/core';
import { useRazorMarkdown } from '@minimact/md';

export function ProductPage() {
  const [productName, setProductName] = useState('Gaming Laptop');
  const [price, setPrice] = useState(1499);
  const [quantity, setQuantity] = useState(8);
  const [features, setFeatures] = useState(['RTX 4080', '32GB RAM', '1TB SSD']);
  const [status, setStatus] = useState('in-stock');

  const [description, setDescription] = useRazorMarkdown(`
# @productName

## Pricing
- **Price:** $@price
- **Quantity:** @quantity
- **Total:** $@(price * quantity) (bulk order)
- **With Tax:** $@(price * 1.1)

@if (quantity > 10) {
🎉 **Bulk Discount Available!**
} else {
⚠️ Limited Stock - Only @quantity remaining
}

## Features
@foreach (var feature in features) {
✓ @feature
}

## Availability
@switch (status) {
  case "in-stock":
    ✅ **In Stock** - Ready to ship!
    break;
  case "low-stock":
    ⚠️ **Low Stock** - Order soon!
    break;
  case "out-of-stock":
    ❌ **Out of Stock** - Check back later
    break;
  default:
    ❓ **Unknown Status**
    break;
}

## Shipping Info
@for (var i = 1; i <= 3; i++) {
**Step @i:** Processing step @i
}
  `);

  return (
    <div className="product-page">
      {description}
      <button onClick={() => alert('Added to cart')}>
        Add to Cart
      </button>
    </div>
  );
}
