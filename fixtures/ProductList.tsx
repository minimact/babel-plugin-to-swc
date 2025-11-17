import { useDomQuery } from 'minimact-query';

export default function ProductList() {
  // Query products with SQL-like syntax
  const query = useDomQuery()
    .from('.product')
    .where(product => product.isIntersecting)
    .orderBy(product => parseFloat(product.attributes['data-price'] || '0'), 'ASC')
    .limit(5);

  // Count total visible products
  const visibleCount = useDomQuery()
    .from('.product')
    .where(p => p.isIntersecting)
    .count();

  // Get average price of visible products
  const avgPrice = useDomQuery()
    .from('.product')
    .where(p => p.isIntersecting)
    .select(p => parseFloat(p.attributes['data-price'] || '0'))
    .avg();

  return (
    <div id="product-list-root">
      <h2 id="product-title">Product List</h2>

      <div id="stats" className="stats">
        <span id="visible-count">Visible: {visibleCount}</span>
        <span id="avg-price">Avg Price: ${avgPrice.toFixed(2)}</span>
      </div>

      <div id="products" className="products">
        {query.select(product => ({
          id: product.attributes['data-id'],
          name: product.textContent,
          price: parseFloat(product.attributes['data-price'] || '0')
        })).map(item => (
          <div key={item.id} className="product-item" data-id={item.id}>
            <span className="name">{item.name}</span>
            <span className="price">${item.price.toFixed(2)}</span>
          </div>
        ))}
      </div>

      {/* Show badge if we have expensive items */}
      {avgPrice > 100 && (
        <span id="premium-badge" className="badge">
          Premium Products
        </span>
      )}
    </div>
  );
}
